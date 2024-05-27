#include <stddef.h>
#include <stdint.h>

uint32_t lookup (const char *str, size_t len) {
    return len + str[0];
}
