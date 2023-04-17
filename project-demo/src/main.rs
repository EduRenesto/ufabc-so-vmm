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
    let swapfile = file_page_loader::SwapFilePageLoader::<256>::new(&"./swapfile.bin").unwrap();

    //let mut mmu = Mmu::<65536, 256, _, _>::new(FIFOPageReplacer::new(), StubPageLoader);
    let mut mmu = Mmu::<512, 2, 256, _, _>::new(FIFOPageReplacer::new(), swapfile);

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
}
