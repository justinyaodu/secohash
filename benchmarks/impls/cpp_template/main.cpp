#include <cstdint>
#include <iostream>
#include <string>

#include "hasher.cpp"

int main() {
    std::ios::sync_with_stdio(false);
    init();
    uint32_t total = 0;
    for (std::string line; std::getline(std::cin, line);) {
        total += lookup(line);
    }
    std::cout << total << std::endl;
}
