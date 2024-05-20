#include <inttypes.h>
#include <stdio.h>
#include <stdint.h>
#include <sys/types.h>

#include "hasher.c"

int main() {
    char* line = NULL;
    size_t line_size = 0;
    ssize_t len;
    uint64_t total = 0;
    while ((len = getline(&line, &line_size, stdin)) != -1) {
        len--;
        line[len] = '\0';
        const struct entry* entry = lookup(line, len);
        if (entry != NULL) {
            total += entry->value;
        }
    }
    printf("%" PRIu64 "\n", total);
}
