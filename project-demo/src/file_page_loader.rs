//! FilePageLoader - struct que carrega/escreve a memória baseada em um arquivo.
//!
//! A estrutura do arquivo é a seguinte:

use std::{fs::File, io::{Read, Seek, SeekFrom, Write}, path::{PathBuf, Path}};

use vm::page_loader::PageLoader;

#[derive(Debug)]
#[repr(C)]
struct SwapFileHeader<const N_PAGES: usize> {
    n_pages: usize,
    page_size: usize,
    indices: [usize; N_PAGES],
}

#[derive(Debug)]
pub struct SwapFilePageLoader<const N_PAGES: usize> {
    file: File,
    header: SwapFileHeader<N_PAGES>,
    page_data: Vec<u8>,
}

impl<const N_PAGES: usize> SwapFilePageLoader<N_PAGES> {
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

    pub fn new<P: AsRef<Path>>(filename: &P) -> std::io::Result<SwapFilePageLoader<N_PAGES>> {
        let mut file = File::options()
            .read(true)
            .write(true)
            .truncate(false)
            .open(filename)?;

        let header = SwapFilePageLoader::parse_header(&mut file)?;

        let mut page_data = vec![0u8; header.n_pages * header.page_size];

        file.read(&mut page_data[..])?;

        let loader = SwapFilePageLoader { file, header, page_data };

        //println!("{:?}", loader);

        Ok(loader)
    }
}

impl<const N_PAGES: usize> PageLoader for SwapFilePageLoader<N_PAGES> {
    fn load_page_into(&mut self, page_number: usize, target: &mut [u8]) {
        println!("load_page_into({:#016X}, {:?})", page_number, target);
        if self.header.indices[page_number] == 0 {
            // 0 significa que a página nao esta presente. No mundo real
            // isso iria causar violação de acesso + crash, mas aqui
            // vamos preencher com 0.

            for i in target {
                *i = 0;
            }

            return;
        }

        let starting_idx = std::mem::size_of::<SwapFileHeader<N_PAGES>>();
        let offset = (self.header.indices[page_number] - 1) * self.header.page_size;

        self.file.seek(SeekFrom::Start((starting_idx + offset).try_into().unwrap())).unwrap();

        self.file.read(target).unwrap();
    }

    fn flush_page(&mut self, page_number: usize, buffer: &[u8]) {
        println!("flush_page({:#016X}, {:?})", page_number, buffer);
        if self.header.indices[page_number] == 0 {
            let offset = std::mem::size_of::<SwapFileHeader<N_PAGES>>();
            self.file.seek(SeekFrom::End(0)).unwrap();
            let cur_position = self.file.stream_position().unwrap();

            let cur_position = cur_position as usize - offset;

            let cur_idx = cur_position / 4;

            let new_idx = cur_idx + 1;

            println!("write {:?} at {}", buffer, self.file.stream_position().unwrap());
            self.file.write(buffer).unwrap();

            self.header.indices[page_number] = new_idx;

            let sz = std::mem::size_of::<usize>();

            let indices_offset = (2 * sz) + (page_number * sz);

            self.file.seek(SeekFrom::Start(indices_offset.try_into().unwrap())).unwrap();
            let bytes = new_idx.to_le_bytes();

            println!("write {:?} at {}", bytes, indices_offset);
            self.file.write(&bytes).unwrap();
        } else {
            let starting_idx = std::mem::size_of::<SwapFileHeader<N_PAGES>>();
            let offset = (self.header.indices[page_number] - 1) * self.header.page_size;

            self.file.seek(SeekFrom::Start((starting_idx + offset).try_into().unwrap())).unwrap();

            println!("write {:?} at {}", buffer, starting_idx + offset);

            self.file.write(buffer).unwrap();
        }
    }
}
