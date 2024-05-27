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
  --slot-name=key
)

{

cat << EOF
struct entry { char* key; uint32_t value; };
%%
EOF

for (( i = 0; ; i++ )); do
  read -r key || break
  cat << EOF
${key},${i}
EOF
done < "${1}"

} | "${gperf_cmd[@]}" > "${project}/hasher.c"

cat >> "${project}/hasher.c" << EOF
uint32_t lookup(const char *key, size_t len) {
    const struct entry* entry = in_word_set(key, len);
    return entry == NULL ? ((uint32_t) -1) : entry->value;
}
EOF

(cd "${project}" && GCC_FLAGS='-Wno-missing-field-initializers' make)

mv "${project}/run" run
#rm -r "${project}"
