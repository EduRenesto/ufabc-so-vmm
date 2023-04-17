#include <stdio.h>
#include <stdint.h>

#define N_PAGES 256
#define PAGE_SIZE 256

struct SwapFileHeader {
    uint64_t n_pages;
    uint64_t page_size;
    uint64_t indices[N_PAGES];
};

int main(int argc, char *argv[]) {
    const char* filename = argv[1];

    FILE *file = fopen(filename, "w");

    struct SwapFileHeader header = {
        .n_pages = N_PAGES,
        .page_size = PAGE_SIZE,
        .indices = { 0 },
    };

    //for (uint64_t i = 0; i < N_PAGES; i++) {
    //    header.indices[i] = i + 1;
    //}

    fwrite(&header, sizeof(struct SwapFileHeader), 1, file);

    fclose(file);

    return 0;
}
