use std::{collections::VecDeque, ops::Range};

use log::{debug, info};

use crate::{
    page_loader::PageLoader,
    page_replacer::{PageEvent, PageReplacer},
    page_table::PageTable,
};

#[derive(Default)]
pub struct MmuStats {
    hits: usize,
    misses: usize,
}

impl MmuStats {
    pub fn print_stats(&self) {
        let total = self.hits + self.misses;
        let miss_rate = self.misses as f32 / total as f32;

        println!("===== Estatísticas da MMU =====");
        println!("Total de acessos: {}", total);
        println!(
            "  Misses: {:>6} ({:>6.2} %)",
            self.misses,
            miss_rate * 100.0
        );
        println!(
            "  Hits:   {:>6} ({:>6.2} %)",
            self.hits,
            (1.0 - miss_rate) * 100.0
        );
    }
}

pub struct Mmu<
    const MEM_SIZE: usize,
    const FRAME_COUNT: usize,
    const PAGE_COUNT: usize,
    REPLACER: PageReplacer,
    LOADER: PageLoader,
> {
    memory: [u8; MEM_SIZE],
    free_frames: VecDeque<usize>,
    page_table: PageTable<PAGE_COUNT>,
    replacer: REPLACER,
    loader: LOADER,
    pub stats: MmuStats,
}

impl<
        const MEM_SIZE: usize,
        const FRAME_COUNT: usize,
        const PAGE_COUNT: usize,
        REPLACER,
        LOADER,
    > Mmu<MEM_SIZE, FRAME_COUNT, PAGE_COUNT, REPLACER, LOADER>
where
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
            stats: MmuStats::default(),
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
                    debug!(
                        "mmu: página {:#06X} suja, salvando antes de sobrescrever",
                        evicted_page_idx
                    );

                    let frame_range = Self::frame_idx_to_range(evicted_page.frame_index);
                    let frame = &self.memory[frame_range];

                    self.loader.flush_page(evicted_page_idx, frame);
                }

                evicted_page.frame_index
            }
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
        let page_offset = address & 0x00FF; // bottom 8 bits

        info!(
            "mmu: acesso addr {:#06X} page_num={:#02X} page_offset={:#02X}",
            address, page_number, page_offset
        );

        let frame_idx = match self.page_table.get(page_number) {
            Some(entry) => {
                debug!("mmu: page hit");
                self.stats.hits += 1;
                entry.frame_index
            }
            None => {
                debug!("mmu: page fault! tratando...");
                self.stats.misses += 1;
                self.handle_page_fault(page_number)
            }
        };

        if mark_dirty {
            self.page_table.mark_dirty(page_number);
        }

        self.replacer.page_event(PageEvent::Touched(page_number));

        let frame_range = Self::frame_idx_to_range(frame_idx);

        debug!(
            "mmu: página {:#02X} mapeada para frame físico idx={:#02X} [{:#02X}; {:#02X})",
            page_number, frame_idx, &frame_range.start, &frame_range.end
        );

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
