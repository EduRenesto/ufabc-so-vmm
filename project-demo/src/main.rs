mod file_page_loader;

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

    dbg!(mmu.read(0xCAFE));
    dbg!(mmu.write(0xCAFE, 0xD));
    //dbg!(mmu.read(0xCAFF));
    dbg!(mmu.read(0xBEEF));
    dbg!(mmu.write(0xBEEF, 0x2));
    //dbg!(mmu.read(0xBEEF));
    dbg!(mmu.read(0xDEAD));
    dbg!(mmu.write(0xDEAD, 0x3));

    dbg!(mmu.read(0xCAFE));
}
