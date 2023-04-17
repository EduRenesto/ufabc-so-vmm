#[derive(Copy, Clone, Default, Debug)]
pub struct PageTableEntry {
    pub frame_index: usize,
    pub dirty: bool,
}

pub struct PageTable<const PAGE_TABLE_SIZE: usize> {
    table: [Option<PageTableEntry>; PAGE_TABLE_SIZE],
}

impl<const PAGE_TABLE_SIZE: usize> PageTable<PAGE_TABLE_SIZE> {
    pub fn new() -> Self {
        PageTable {
            table: [None; PAGE_TABLE_SIZE],
        }
    }

    pub fn set(&mut self, page_number: usize, frame_index: usize) {
        self.table[page_number] = Some(PageTableEntry { frame_index, dirty: false });
    }

    pub fn get(&self, page_number: usize) -> Option<PageTableEntry> {
        self.table[page_number]
    }

    pub fn mark_dirty(&mut self, idx: usize) {
        let page = self.table[idx].as_mut().unwrap();

        page.dirty = true;
    }
}
