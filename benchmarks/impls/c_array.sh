#!/bin/bash

set -eu

project='c_template'
cp -a "$(dirname "${0}")/${project}" .

{

cat << EOF
#include <string.h>

struct entry { char* name; size_t len; uint64_t value; };

uint64_t lookup (const char *str, size_t len) {
    static const struct entry entries[] = {
EOF

for (( i = 1; ; i++ )); do
  read -r key || break
  cat << EOF
        {"${key}", ${#key}, ${i}},
EOF
done < "${1}"

cat << EOF
    };

    for (size_t i = 0; i < sizeof(entries) / sizeof(entries[0]); i++) {
        if (len == entries[i].len && memcmp(str, entries[i].name, len) == 0) {
            return entries[i].value;
        }
    }
    return 0;
}
EOF

} > "${project}/hasher.c"

(cd "${project}" && GCC_FLAGS='-Wno-unused-parameter' make)

mv "${project}/run" run
rm -r "${project}"
