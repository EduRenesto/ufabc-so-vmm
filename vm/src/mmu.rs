use std::{collections::VecDeque, ops::Range};

#[derive(Copy, Clone, Default, Debug)]
struct PageTableEntry {
    frame_index: usize,
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

    fn set(&mut self, page_number: usize, frame_index: usize) {
        self.table[page_number] = Some(PageTableEntry { frame_index, dirty: false });
    }

    fn get(&self, page_number: usize) -> Option<PageTableEntry> {
        self.table[page_number]
    }

    fn mark_dirty(&mut self, idx: usize) {
        let page = self.table[idx].as_mut().unwrap();

        page.dirty = true;
    }
}

pub enum PageEvent {
    Touched(usize),
    Loaded(usize),
}

pub trait PageReplacer {
    fn page_event(&mut self, _event: PageEvent) { }

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
    fn page_event(&mut self, event: PageEvent) {
        if let PageEvent::Loaded(idx) = event {
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

pub struct Mmu<
    const MEM_SIZE: usize,
    const FRAME_COUNT: usize,
    const PAGE_COUNT: usize,
    REPLACER: PageReplacer,
    LOADER: PageLoader
> {
    memory: [u8; MEM_SIZE],
    free_frames: VecDeque<usize>,
    page_table: PageTable<PAGE_COUNT>,
    replacer: REPLACER,
    loader: LOADER,
}

impl<const MEM_SIZE: usize, const FRAME_COUNT: usize, const PAGE_COUNT: usize, REPLACER, LOADER> Mmu<MEM_SIZE, FRAME_COUNT, PAGE_COUNT, REPLACER, LOADER> where
    REPLACER: PageReplacer,
    LOADER: PageLoader,
{
    pub fn new(replacer: REPLACER, loader: LOADER) -> Self {
        let free_frames = (0..FRAME_COUNT).into_iter().collect();

        Mmu {
            memory: [0; MEM_SIZE],
            free_frames,
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
        let frame_idx = match self.free_frames.pop_front() {
            Some(empty_idx) => empty_idx,
            None => {
                let evicted_page_idx = self.replacer.pick_replacement_page();

                let evicted_page = self.page_table.get(evicted_page_idx).unwrap();

                if evicted_page.dirty {
                    println!("mmu: página {:#06X} suja, salvando antes de sobrescrever", evicted_page_idx);

                    let frame_range = Self::frame_idx_to_range(evicted_page.frame_index);
                    let frame = &self.memory[frame_range];

                    self.loader.flush_page(evicted_page_idx, frame);
                }

                evicted_page.frame_index
            },
        };

        self.page_table.set(page_number, frame_idx);

        let frame_range = Self::frame_idx_to_range(frame_idx);

        let frame = &mut self.memory[frame_range];

        self.loader.load_page_into(page_number, frame);

        self.replacer.page_event(PageEvent::Loaded(page_number));

        frame_idx
    }

    fn translate_addr(&mut self, address: usize, mark_dirty: bool) -> (Range<usize>, usize) {
        let address = address & 0xFFFF; // trunca o endereco para 16 bits

        let page_number = (address & 0xFF00) >> 8; // top 8 bits
        let page_offset = address & 0x00FF;        // bottom 8 bits

        println!("mmu: acesso addr {:#06X} page_num={:#02X} page_offset={:#02X}", address, page_number, page_offset);

        let frame_idx = match self.page_table.get(page_number) {
            Some(entry) => {
                println!("mmu: page hit");
                entry.frame_index
            },
            None => {
                println!("mmu: page fault! tratando...");
                self.handle_page_fault(page_number)
            },
        };

        if mark_dirty { self.page_table.mark_dirty(page_number); }

        self.replacer.page_event(PageEvent::Touched(page_number));

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
