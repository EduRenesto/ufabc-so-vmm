pub trait PageLoader {
    fn load_page_into(&mut self, page_number: usize, target: &mut [u8]);

    fn flush_page(&mut self, page_number: usize, buffer: &[u8]);
}
