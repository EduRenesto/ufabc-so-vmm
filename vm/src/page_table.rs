/// Uma entrada na Page Table.
#[derive(Copy, Clone, Default, Debug)]
pub struct PageTableEntry {
    /// O índice do frame no qual esta página está carregada.
    pub frame_index: usize,
    /// Indica se houveram alterações na página que devem ser reescritas
    /// no disco.
    pub dirty: bool,
}

/// Um wrapper sobre a Page Table.
pub struct PageTable<const PAGE_TABLE_SIZE: usize> {
    /// A Page Table. Se table[page_number] é um None, a página é inválida
    /// e deve ser carregada; se é Some(_), é válida e pode ser usada.
    table: [Option<PageTableEntry>; PAGE_TABLE_SIZE],
}

impl<const PAGE_TABLE_SIZE: usize> PageTable<PAGE_TABLE_SIZE> {
    /// Constrói uma nova page table vazia.
    pub fn new() -> Self {
        PageTable {
            table: [None; PAGE_TABLE_SIZE],
        }
    }

    /// Atualiza um item na page table.
    pub fn set(&mut self, page_number: usize, frame_index: usize) {
        self.table[page_number] = Some(PageTableEntry {
            frame_index,
            dirty: false,
        });
    }

    /// Busca um item na page table.
    pub fn get(&self, page_number: usize) -> Option<PageTableEntry> {
        self.table[page_number]
    }

    /// Invalida uma página.
    pub fn invalidate(&mut self, page_number: usize) {
        self.table[page_number] = None;
    }

    /// Marca uma página como dirty.
    pub fn mark_dirty(&mut self, idx: usize) {
        let page = self.table[idx].as_mut().unwrap();

        page.dirty = true;
    }
}
