use std::{collections::VecDeque, ops::Range};

pub struct PageTable<const PAGE_TABLE_SIZE: usize> {
    table: [Option<usize>; PAGE_TABLE_SIZE],
}

impl<const PAGE_TABLE_SIZE: usize> PageTable<PAGE_TABLE_SIZE> {
    fn new() -> Self {
        PageTable {
            table: [None; PAGE_TABLE_SIZE],
        }
    }

    fn lookup(&self, page_number: usize) -> Option<usize> {
        self.table.iter().position(|entry| entry == &Some(page_number))
    }

    fn set(&mut self, idx: usize, page_number: usize) {
        self.table[idx] = Some(page_number);
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
    fn load_page_into(&self, target: &mut [u8]);
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

        self.page_table.set(frame_idx, page_number);

        let frame_range = Self::frame_idx_to_range(frame_idx);

        let frame = &mut self.memory[frame_range];

        self.loader.load_page_into(frame);

        self.replacer.frame_event(FrameEvent::Loaded(frame_idx));

        frame_idx
    }

    fn read(&mut self, address: usize) -> u8 {
        let address = address & 0xFFFF; // trunca o endereco para 16 bits

        let page_number = address & 0xFF00; // top 8 bits
        let page_offset = address & 0x00FF; // bottom 8 bits

        let frame_idx = match self.page_table.lookup(page_number) {
            Some(frame_idx) => frame_idx,
            None => self.handle_page_fault(page_number),
        };

        self.replacer.frame_event(FrameEvent::Touched(frame_idx));

        let frame = &mut self.memory[Self::frame_idx_to_range(frame_idx)];

        frame[page_offset]
    }
}
