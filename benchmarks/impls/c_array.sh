#!/bin/bash

set -eu

project='c_template'
cp -a "$(dirname "${0}")/${project}" .

{

cat << EOF
#include <string.h>

struct entry { char* name; uint64_t value; };

const struct entry* lookup (const char *str, size_t len) {
    static const struct entry entries[] = {
EOF

for (( i = 1; ; i++ )); do
  read -r key || break
  cat << EOF
        {"${key}", ${i}},
EOF
done < "${1}"

cat << EOF
    };

    for (size_t i = 0; i < sizeof(entries) / sizeof(entries[0]); i++) {
        // if (*str == *entries[i].name && strcmp(str + 1, entries[i].name + 1) == 0) {
        if (strcmp(str, entries[i].name) == 0) {
            return &entries[i];
        }
    }
    return 0;
}
EOF

} > "${project}/hasher.c"

(cd "${project}" && GCC_FLAGS='-Wno-unused-parameter' make)

mv "${project}/run" run
rm -r "${project}"
