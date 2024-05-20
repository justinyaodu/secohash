#!/bin/bash

set -eu

project='c_template'
cp -a "$(dirname "${0}")/${project}" .

gperf_cmd=(
  gperf
  --language=ANSI-C
  --struct-type
  --readonly-tables
  --includes
)

{

cat << EOF
struct entry { char* name; uint64_t value; };
%%
EOF

for (( i = 1; ; i++ )); do
  read -r key || break
  cat << EOF
${key},${i}
EOF
done < "${1}"

} | "${gperf_cmd[@]}" > "${project}/hasher.c"

cat >> "${project}/hasher.c" << EOF
uint64_t lookup(const char *str, size_t len) {
    const struct entry* entry = in_word_set(str, len);
    return entry == NULL ? 0 : entry->value;
}
EOF

(cd "${project}" && GCC_FLAGS='-Wno-missing-field-initializers' make)

mv "${project}/run" run
#rm -r "${project}"
