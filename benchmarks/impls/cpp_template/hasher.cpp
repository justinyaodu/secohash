#include <cstdint>
#include <string>

void init() {}

uint32_t lookup(const std::string& str) {
    return str.length() + str[0];
}
