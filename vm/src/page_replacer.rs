use std::collections::VecDeque;

/// Um evento de uma página, disparado pela Mmu.  O algoritmo replacer pode ou
/// não usar esses eventos para seus cálculos.
pub enum PageEvent {
    /// A página foi tocada (leitura ou escrita).
    Touched(usize),
    /// A página foi carregada do disco.
    Loaded(usize),
}

/// A interface do algoritmo de substituição de página.
pub trait PageReplacer {
    /// Avia ao replacer que houve um evento de página.
    fn page_event(&mut self, _event: PageEvent) {}

    /// Funcão principal da interface: escolhe uma página
    /// a ser substituída.
    fn pick_replacement_page(&mut self) -> usize;
}

/// Implementação do algoritmo FIFO de substituição.
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
            // Assim que a página foi carregada, a insira no fim da fila.
            self.fifo.push_back(idx)
        }
    }

    fn pick_replacement_page(&mut self) -> usize {
        // Pegue a página no começo da fila. Ela será a que foi carregada há
        // mais tempo.
        self.fifo.pop_front().unwrap()
    }
}
