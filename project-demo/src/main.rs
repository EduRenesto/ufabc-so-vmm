//! Ponto de entrada da aplicação demo do projeto.
//!
//! Toda a implementação interessante do projeto foi feita nos módulos
//! na crate `vm` -- por favor, visite os arquivos e leia os comentários!
//!
//! Este arquivo apenas contém o código de instanciamento da estrutura da Mmu
//! e o handling da entrada padrão.
//!
//! ## Entrada
//!
//! Este programa espera uma entrada linha-a-linha, onde cada linha é um
//! comando dos seguintes:
//!
//! - `r <address>`: lê o byte no endereço `<address>` e apresenta na stdout;
//! - `w <address> <byte>`: escreve o byte `<byte>` em `<address>`;
//!
//! Note que todos os valores *são em hexadecimal*. Outros valores causarão um
//! panic na aplicação.
//!
//! ### Exemplo
//!
//! ```
//! r 0xCAFE
//! w 0xCAFE 0xA
//! w 0xCAFF 0xB
//! r 0xBABE
//! w 0xDEAD 0x1
//! ```

mod file_page_loader;

use std::io::BufRead;

use vm::{mmu::Mmu, page_loader::PageLoader, page_replacer::FIFOPageReplacer};

struct StubPageLoader;

impl PageLoader for StubPageLoader {
    fn load_page_into(&mut self, page_loader: usize, target: &mut [u8]) {
        for i in target {
            *i = (page_loader & 0xF) as u8;
        }
    }

    fn flush_page(&mut self, page_number: usize, buffer: &[u8]) {
        println!(
            "stub_page_loader: write page {:#06X} {:?}",
            page_number, buffer
        );
    }
}

fn main() {
    env_logger::init();

    let swapfile = file_page_loader::SwapFilePageLoader::<256>::new(&"./swapfile.bin").unwrap();

    // Cria uma MMU com:
    // - 65536 bytes (64kb) de memória...;
    // - ...divididos em 256 frames...;
    // - ...populados por 256 páginas.
    let mut mmu = Mmu::<65536, 256, 256, _, _>::new(FIFOPageReplacer::new(), swapfile);

    // Utilize essa construção para modificar o arquivo swap (veja README.md)
    //let mut mmu = Mmu::<256, 1, 256, _, _>::new(FIFOPageReplacer::new(), swapfile);

    let mut stdin = std::io::stdin().lock();
    let mut line = String::new();

    while let Ok(_) = stdin.read_line(&mut line) {
        let mut tokens = line.split(" ");

        let cmd = tokens.next().unwrap_or("INVALID");

        match cmd {
            "r" => {
                let address = tokens.next().unwrap().trim();
                let address = usize::from_str_radix(&address[2..], 16).unwrap();

                let value = mmu.read(address);

                println!("{:#06X} => {:#X}", address, value);
            }
            "w" => {
                let address = tokens.next().unwrap().trim();
                let address = usize::from_str_radix(&address[2..], 16).unwrap();

                let value = tokens.next().unwrap().trim();
                let value = u8::from_str_radix(&value[2..], 16).unwrap();

                mmu.write(address, value);
            }
            "" => {
                break;
            }
            _ => {
                println!("comando inválido: {}", cmd);
            }
        }

        line.clear();
    }

    mmu.stats.print_stats();
}
