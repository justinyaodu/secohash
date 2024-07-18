#include <cstdint>
#include <iostream>
#include <string>
#include <string_view>
#include <stdio.h>

#include "hasher.cpp"

int main() {
    init();
    char* line = NULL;
    size_t line_size = 0;
    ssize_t len;
    uint32_t total = 0;
    while ((len = getline(&line, &line_size, stdin)) != -1) {
        len--;
        line[len] = '\0';
        total += lookup(std::string_view(line, len));
    }
    std::cout << total << std::endl;
}
