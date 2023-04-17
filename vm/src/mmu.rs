//! MMU: ponto principal da implementação do sistema de memória
//! virtual.
//!
//! Esse módulo implementa a lógica principal de gerenciamento de memória,
//! terceirizando alguns comportamentos para módulos adjacentes.

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

/// Uma struct parametrizada pelo tamanho da memória, pelo número de frames,
/// pelo número de páginas e pelos tipos do carregador de páginas e da política
/// de substituição de páginas.
pub struct Mmu<
    const MEM_SIZE: usize,
    const FRAME_COUNT: usize,
    const PAGE_COUNT: usize,
    REPLACER: PageReplacer,
    LOADER: PageLoader,
> {
    /// Um array de MEM_SIZE bytes representa a memória.
    memory: [u8; MEM_SIZE],
    /// Uma fila de frames ainda não alocados na memória principal.
    free_frames: VecDeque<usize>,
    /// A page table.
    page_table: PageTable<PAGE_COUNT>,
    /// A implementação da política de substituição.
    replacer: REPLACER,
    /// A implementação do carregador de páginas.
    loader: LOADER,
    /// Instância de monitoramento de estatísticas.
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
    /// Constrói uma nova instância de Mmu.
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

    /// Converte um índice de frame num range que pode ser utilizado
    /// para indexar a array memory.
    fn frame_idx_to_range(frame_idx: usize) -> Range<usize> {
        let frame_size = MEM_SIZE / FRAME_COUNT;

        Range {
            start: frame_idx * frame_size,
            end: (frame_idx + 1) * frame_size,
        }
    }

    /// Faz o tratamento de uma page fault.
    fn handle_page_fault(&mut self, page_number: usize) -> usize {
        // Aqui, inicialmente vamos escolher em qual frame carregar a página.
        // Tenta pegar um frame que ainda não foi utilizado.
        let frame_idx = match self.free_frames.pop_front() {
            // Se conseguiu, retorna seu índice imediatamente, e vamos utilizá-lo.
            Some(empty_idx) => empty_idx,
            None => {
                // Se não há frames vazios, vamos escolher uma página para ser substituída.
                // Para isso, vamos chamar o nosso replacer.
                let evicted_page_idx = self.replacer.pick_replacement_page();

                // Olhamos para dentro da entrada da page table desta página, e verificamos
                // se a página está dirty. Se sim, então nós vamos chamar nosso loader
                // para fazer o flush de volta para disco.
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

                let idx = evicted_page.frame_index;

                // Invalida a página na page table.
                self.page_table.invalidate(page_number);

                // E finalmente retornamos o frame no qual essa página estava guardada.
                idx
            }
        };

        // Já que temos o frame, atualizamos a entrada na page table.
        self.page_table.set(page_number, frame_idx);

        // Olhamos para a janela na memória que é o frame.
        let frame_range = Self::frame_idx_to_range(frame_idx);
        let frame = &mut self.memory[frame_range];

        // Chama o loader para carregar a página no frame.
        self.loader.load_page_into(page_number, frame);

        // Avisa o replacer, que pode usar esse evento para seus cálculos.
        self.replacer.page_event(PageEvent::Loaded(page_number));

        // Retorna o índice do frame.
        frame_idx
    }

    // Função principal que faz a translação entre um endereço virtual e um
    // endereço físico (no nosso caso, modelado por um range dentro da array de
    // memória e um offset dentro desse range).
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
                // Se houve page hit, já sabemos imediatamente qual o frame
                // que queremos acessar.
                debug!("mmu: page hit");
                self.stats.hits += 1;
                entry.frame_index
            }
            None => {
                // Se houve page fault, vamos escolher qual o frame será carregado,
                // e vamos carregar a página nele.
                debug!("mmu: page fault! tratando...");
                self.stats.misses += 1;
                self.handle_page_fault(page_number)
            }
        };

        // Quando a ação é uma escrita, também vamos marcar a dirty flag
        // para que a página seja reescrita de volta em disco.
        if mark_dirty {
            self.page_table.mark_dirty(page_number);
        }

        // Emite um evento para cálculo do replacer.
        self.replacer.page_event(PageEvent::Touched(page_number));

        // Calcula a janela do frame dentro da array memória.
        let frame_range = Self::frame_idx_to_range(frame_idx);

        debug!(
            "mmu: página {:#02X} mapeada para frame físico idx={:#02X} [{:#02X}; {:#02X})",
            page_number, frame_idx, &frame_range.start, &frame_range.end
        );

        // Retorna o frame e o offset.
        (frame_range, page_offset)
    }

    /// Lê o byte existente no endereço address.
    pub fn read(&mut self, address: usize) -> u8 {
        // Faz a tradução do endereço.
        let (frame_range, page_offset) = self.translate_addr(address, false);

        // Olha na array memory a partir da janela (que corresponde ao frame da página).
        let frame = &mut self.memory[frame_range];

        // Olha no frame considerando o offset, que é exatamente o endereço desejado.
        frame[page_offset]
    }

    /// Escreve um byte value no endereço address.
    pub fn write(&mut self, address: usize, value: u8) {
        // Faz a tradução do endereço.
        let (frame_range, page_offset) = self.translate_addr(address, true);

        // Olha na array memory a partir da janela (que corresponde ao frame da página).
        let frame = &mut self.memory[frame_range];

        // Escreve no frame considerando o offset, que é exatamente o endereço desejado.
        frame[page_offset] = value;
    }
}
