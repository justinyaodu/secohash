#include <cstdint>
#include <string>

void init() {}

uint32_t lookup(std::string_view str) {
    return str.length() + str[0];
}
