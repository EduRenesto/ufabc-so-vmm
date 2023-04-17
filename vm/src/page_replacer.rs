use std::collections::VecDeque;

pub enum PageEvent {
    Touched(usize),
    Loaded(usize),
}

pub trait PageReplacer {
    fn page_event(&mut self, _event: PageEvent) {}

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
