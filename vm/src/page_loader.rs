/// Interface do carregador de páginas.
pub trait PageLoader {
    /// Carrega uma página do disco em memória.
    fn load_page_into(&mut self, page_number: usize, target: &mut [u8]);

    /// Faz o writeback de uma página de volta para o disco.
    fn flush_page(&mut self, page_number: usize, buffer: &[u8]);
}
