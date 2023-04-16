use vm::mmu::{PageLoader, Mmu, FIFOPageReplacer};

struct StubPageLoader;

impl PageLoader for StubPageLoader {
    fn load_page_into(&self, page_loader: usize, target: &mut [u8])
    {
        for i in target {
            *i = (page_loader & 0xF) as u8;
        }
    }

    fn flush_page(&self, page_number: usize, buffer: &[u8]) {
        println!("stub_page_loader: write page {:#06X} {:?}", page_number, buffer);
    }
}

fn main() {
    //let mut mmu = Mmu::<65536, 256, _, _>::new(FIFOPageReplacer::new(), StubPageLoader);
    let mut mmu = Mmu::<512, 2, 256, _, _>::new(FIFOPageReplacer::new(), StubPageLoader);

    dbg!(mmu.read(0xCAFE));
    dbg!(mmu.write(0xCAFE, 0x0));
    dbg!(mmu.read(0xCAFE));
    dbg!(mmu.read(0xBEEF));
    dbg!(mmu.read(0xCAFE));
    dbg!(mmu.read(0xBEEF));
}
