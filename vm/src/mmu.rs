use std::{collections::VecDeque, ops::Range};

#[derive(Copy, Clone, Default, Debug)]
struct PageTableEntry {
    page_number: usize,
    dirty: bool,
}

pub struct PageTable<const PAGE_TABLE_SIZE: usize> {
    table: [Option<PageTableEntry>; PAGE_TABLE_SIZE],
}

impl<const PAGE_TABLE_SIZE: usize> PageTable<PAGE_TABLE_SIZE> {
    fn new() -> Self {
        PageTable {
            table: [None; PAGE_TABLE_SIZE],
        }
    }

    fn lookup(&self, page_number: usize) -> Option<usize> {
        self.table.iter().map(|entry| entry.map(|x| x.page_number)).position(|entry| entry == Some(page_number))
    }

    fn set(&mut self, idx: usize, page_number: usize) {
        self.table[idx] = Some(PageTableEntry { page_number, dirty: false });
    }

    fn get(&self, idx: usize) -> Option<PageTableEntry> {
        self.table[idx]
    }

    fn mark_dirty(&mut self, idx: usize) {
        let page = self.table[idx].as_mut().unwrap();

        page.dirty = true;
    }

    fn find_free_entry(&self) -> Option<usize> {
        self.table.iter().position(|entry| entry.is_none())
    }
}

pub enum FrameEvent {
    Touched(usize),
    Loaded(usize),
}

pub trait PageReplacer {
    fn frame_event(&mut self, _event: FrameEvent) { }

    fn pick_replacement_page(&mut self) -> usize;
}

pub struct FIFOPageReplacer {
    fifo: VecDeque<usize>,
}

impl FIFOPageReplacer {
    pub fn new() -> Self {
        FIFOPageReplacer {
            fifo: VecDeque::new(),
        }
    }
}

impl PageReplacer for FIFOPageReplacer {
    fn frame_event(&mut self, event: FrameEvent) {
        if let FrameEvent::Loaded(idx) = event {
            self.fifo.push_back(idx)
        }
    }

    fn pick_replacement_page(&mut self) -> usize {
        self.fifo.pop_front().unwrap()
    }
}

pub trait PageLoader {
    fn load_page_into(&self, page_number: usize, target: &mut [u8]);

    fn flush_page(&self, page_number: usize, buffer: &[u8]);
}

pub struct Mmu<const MEM_SIZE: usize, const FRAME_COUNT: usize, REPLACER: PageReplacer, LOADER: PageLoader> {
    memory: [u8; MEM_SIZE],
    page_table: PageTable<FRAME_COUNT>,
    replacer: REPLACER,
    loader: LOADER,
}

impl<const MEM_SIZE: usize, const FRAME_COUNT: usize, REPLACER, LOADER> Mmu<MEM_SIZE, FRAME_COUNT, REPLACER, LOADER> where
    REPLACER: PageReplacer,
    LOADER: PageLoader,
{
    pub fn new(replacer: REPLACER, loader: LOADER) -> Self {
        Mmu {
            memory: [0; MEM_SIZE],
            page_table: PageTable::new(),
            replacer,
            loader,
        }
    }

    fn frame_idx_to_range(frame_idx: usize) -> Range<usize> {
        let frame_size = MEM_SIZE / FRAME_COUNT;

        Range {
            start: frame_idx * frame_size,
            end: (frame_idx + 1) * frame_size,
        }
    }

    fn handle_page_fault(&mut self, page_number: usize) -> usize {
        let frame_idx = match self.page_table.find_free_entry() {
            Some(empty_idx) => empty_idx,
            None => self.replacer.pick_replacement_page(),
        };

        if let Some(page) = self.page_table.get(frame_idx) {
            if page.dirty {
                println!("mmu: página {:#06X} suja, salvando antes de sobrescrever", page.page_number);

                let frame_range = Self::frame_idx_to_range(frame_idx);
                let frame = &self.memory[frame_range];

                self.loader.flush_page(page.page_number, frame);
            }
        }

        self.page_table.set(frame_idx, page_number);

        let frame_range = Self::frame_idx_to_range(frame_idx);

        let frame = &mut self.memory[frame_range];

        self.loader.load_page_into(page_number, frame);

        self.replacer.frame_event(FrameEvent::Loaded(frame_idx));

        frame_idx
    }

    fn translate_addr(&mut self, address: usize, mark_dirty: bool) -> (Range<usize>, usize) {
        let address = address & 0xFFFF; // trunca o endereco para 16 bits

        let page_number = (address & 0xFF00) >> 8; // top 8 bits
        let page_offset = address & 0x00FF;        // bottom 8 bits

        println!("mmu: acesso addr {:#06X} page_num={:#02X} page_offset={:#02X}", address, page_number, page_offset);

        let frame_idx = match self.page_table.lookup(page_number) {
            Some(frame_idx) => {
                println!("mmu: page hit");
                frame_idx
            },
            None => {
                println!("mmu: page fault! tratando...");
                self.handle_page_fault(page_number)
            },
        };

        if mark_dirty { self.page_table.mark_dirty(frame_idx); }

        self.replacer.frame_event(FrameEvent::Touched(frame_idx));

        let frame_range = Self::frame_idx_to_range(frame_idx);

        println!("mmu: página {:#02X} mapeada para frame físico idx={:#02X} [{:#02X}; {:#02X})", page_number, frame_idx, &frame_range.start, &frame_range.end);

        (frame_range, page_offset)
    }

    pub fn read(&mut self, address: usize) -> u8 {
        let (frame_range, page_offset) = self.translate_addr(address, false);

        let frame = &mut self.memory[frame_range];

        frame[page_offset]
    }

    pub fn write(&mut self, address: usize, value: u8) {
        let (frame_range, page_offset) = self.translate_addr(address, true);

        let frame = &mut self.memory[frame_range];

        frame[page_offset] = value;
    }
}
