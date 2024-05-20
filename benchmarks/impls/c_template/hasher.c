#include <string.h>

struct entry { char* name; uint64_t value; };

struct entry the_only_entry = { NULL, 0 };

const struct entry* lookup (const char *str, size_t len) {
    the_only_entry.value = len + str[0];
    return &the_only_entry;
}
