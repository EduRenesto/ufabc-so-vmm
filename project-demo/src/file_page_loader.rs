//! SwapFilePageLoader - Implementação do PageLoader que utiliza um arquivo
//! no sistema de arquivos como fonte.
//!
//! Eu poderia ter usado JSON, YAML ou algo mais simples, mas claro que gosto
//! de deixar as coisas mais difíceis para mim e resolvi criar uma estrutura
//! binária no arquivo...
//!
//! O arquivo "swap" contém um header seguido de uma lista não ordenada de
//! páginas.
//!
//! O header contém uma lista que associa cada page number a um índice
//! na lista de páginas. Nessa lista de índices, 0 representa que a página
//! não está presente no arquivo, e i != 0 representa que a página está na (i-1)-ésima
//! posição na lista de dados brutos ao final do arquivo.
//!
//! Então, a busca no arquivo se faz da seguinte maneira, assumindo que queremos
//! carregar a página `page_number`:
//!
//! 1. Carregamos o header;
//! 2. Olhamos para a `page_number`-ésima posição na lista `indices`;
//! 3. Se o item na lista é 0, então a página não está no arquivo (e nesse caso
//!    retornamos a página vazia, por escolha -- no mundo real isso causaria um
//!    crash).
//! 4. Se o item na lista é `i`, caminhamos até o primeiro byte depois do fim do
//!    header e demos caminhamos mais `(i - 1) * page_size` bytes;
//! 5. Lemos `page_size` bytes contíguos a partir da posição atual para o buffer
//!    desejado, que no final da call stack será o frame na array de memória (como
//!    escrevemos na mmu).
//!
//! O passo de escrita é parecido, mas também precisamos atualizar a lista de índices.
//!
//! Em suma, a estrutura do arquivo é a seguinte:
//!
//! | descrição         | tamanho                |
//! |-------------------|------------------------|
//! | header            | 16 + n_pages * 8 bytes |
//! | página i_0        | page_size bytes        |
//! | página i_1        | page_size bytes        |
//! | ...               | ...                    |
//! | página i_N        | page_size bytes        |
//!
//! E o header tem a seguinte estrutura:
//!
//! | descrição              | tamanho           |
//! |------------------------|-------------------|
//! | número de páginas      | 8 bytes           |
//! | tamanho de cada página | 8 bytes           |
//! | indices das páginas    | n_pages * 8 bytes |
//!
//! ---
//!
//! Exagerei? *Sim*. :P

use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

use vm::page_loader::PageLoader;

/// O header do swap file.
#[derive(Debug)]
#[repr(C)]
struct SwapFileHeader<const N_PAGES: usize> {
    /// Número de páginas total. Usado como *sanity check*.
    n_pages: usize,
    /// O tamanho de cada página.
    page_size: usize,
    /// O índice de cada página na seção de dados do arquivo.
    indices: [usize; N_PAGES],
}

/// O carregador que lê do arquivo.
#[derive(Debug)]
pub struct SwapFilePageLoader<const N_PAGES: usize> {
    /// O arquivo fonte.
    file: File,
    /// Cópia do header.
    header: SwapFileHeader<N_PAGES>,
}

impl<const N_PAGES: usize> SwapFilePageLoader<N_PAGES> {
    /// Lê o header e o interpreta.
    fn parse_header(file: &mut File) -> std::io::Result<SwapFileHeader<N_PAGES>> {
        let mut n_pages_buf = vec![0u8; std::mem::size_of::<usize>()];
        file.read(&mut n_pages_buf[..])?;
        let n_pages = usize::from_le_bytes(n_pages_buf.try_into().unwrap());
        assert_eq!(n_pages, N_PAGES);

        let mut page_size_buf = vec![0u8; std::mem::size_of::<usize>()];
        file.read(&mut page_size_buf[..])?;
        let page_size = usize::from_le_bytes(page_size_buf.try_into().unwrap());

        let mut indices_buf = vec![0; n_pages * std::mem::size_of::<usize>()];

        file.read_exact(&mut indices_buf[..])?;

        let mut indices = [usize::MAX; N_PAGES];

        for (chunk_idx, chunk) in indices_buf.chunks(std::mem::size_of::<usize>()).enumerate() {
            indices[chunk_idx] = usize::from_le_bytes(chunk.try_into().unwrap());
        }

        Ok(SwapFileHeader {
            n_pages,
            page_size,
            indices,
        })
    }

    //// Constrói um novo loader.
    pub fn new<P: AsRef<Path>>(filename: &P) -> std::io::Result<SwapFilePageLoader<N_PAGES>> {
        let mut file = File::options()
            .read(true)
            .write(true)
            .truncate(false)
            .open(filename)?;

        let header = SwapFilePageLoader::parse_header(&mut file)?;

        let loader = SwapFilePageLoader { file, header };

        Ok(loader)
    }
}

impl<const N_PAGES: usize> PageLoader for SwapFilePageLoader<N_PAGES> {
    fn load_page_into(&mut self, page_number: usize, target: &mut [u8]) {
        if self.header.indices[page_number] == 0 {
            // 0 significa que a página nao esta presente. No mundo real
            // isso iria causar violação de acesso + crash, mas aqui
            // vamos preencher com 0.

            for i in target {
                *i = 0;
            }

            return;
        }

        // A partir da lista de índices, calcula a posição do começo da página
        // na seção de dados do arquivo. A seção começa no primeiro byte
        // depois do header, e cada entrada na seção tem page_size bytes,
        // então queremos sizeof(header) + index[page_number] * page_size.
        let starting_idx = std::mem::size_of::<SwapFileHeader<N_PAGES>>();
        let offset = (self.header.indices[page_number] - 1) * self.header.page_size;

        self.file
            .seek(SeekFrom::Start((starting_idx + offset).try_into().unwrap()))
            .unwrap();

        // Depois de encontrar, apenas lemos page_size bytes contíguos.
        self.file.read(target).unwrap();
    }

    fn flush_page(&mut self, page_number: usize, buffer: &[u8]) {
        // Essa função é meio... macarronada.
        // Eu poderia refatorar ela, mas estou sem tempo :(

        if self.header.indices[page_number] == 0 {
            // Nesse caso, a página nunca foi carregada do arquivo, então
            // precisamos criar mais uma entrada.
            //
            // Primeiro descobrimos qual a posição da última página gravada no
            // arquivo, criamos uma depois, e atualizamos o índice na lista de índices.
            //
            // Mas temos que fazer tudo isso escovando bytes.

            let offset = std::mem::size_of::<SwapFileHeader<N_PAGES>>();
            self.file.seek(SeekFrom::End(0)).unwrap();
            let cur_position = self.file.stream_position().unwrap();

            let cur_position = cur_position as usize - offset;

            let cur_idx = cur_position / 4;

            let new_idx = cur_idx + 1;

            self.file.write(buffer).unwrap();

            self.header.indices[page_number] = new_idx;

            let sz = std::mem::size_of::<usize>();

            let indices_offset = (2 * sz) + (page_number * sz);

            self.file
                .seek(SeekFrom::Start(indices_offset.try_into().unwrap()))
                .unwrap();
            let bytes = new_idx.to_le_bytes();

            self.file.write(&bytes).unwrap();
        } else {
            // Aqui é mais fácil -- a página já existe no arquivo. Vamos só atualizar
            // a seção de dados calculando sua posição no arquivo e sobrescrevendo page_size
            // bytes contíguos a partir do buffer dado.

            let starting_idx = std::mem::size_of::<SwapFileHeader<N_PAGES>>();
            let offset = (self.header.indices[page_number] - 1) * self.header.page_size;

            self.file
                .seek(SeekFrom::Start((starting_idx + offset).try_into().unwrap()))
                .unwrap();

            self.file.write(buffer).unwrap();
        }
    }
}
