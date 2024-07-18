#!/bin/bash

set -eu

project='cpp_template'
cp -a "$(dirname "${0}")/${project}" .

{

cat << EOF
#include <cstdint>
#include <string>
#include <unordered_map>

std::unordered_map<std::string, uint32_t> map;

void init() {
EOF


for (( i = 0; ; i++ )); do
    read -r key || break
    printf '    map["%s"] = %d;\n' "${key}" "${i}"
done < "${1}"

echo "${i}" > "hash_table_size"
echo 0 > "data_bytes"

cat << EOF
};

uint32_t lookup(const std::string& str) {
    auto found = map.find(str);
    if (found != map.end()) {
        return found->second;
    } else {
        return -1;
    }
}
EOF

} > "${project}/hasher.cpp"

(cd "${project}" && make)

mv "${project}/run" run
#rm -r "${project}"
