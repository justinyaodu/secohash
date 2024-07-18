#!/bin/bash

set -eu

project='c_template'
cp -a "$(dirname "${0}")/${project}" .

gperf_cmd=(
  gperf
  --language=ANSI-C
  --struct-type
  --readonly-tables
  --compare-lengths
  --includes
  --multiple-iterations=10
  --slot-name=key
  --delimiters=$'\t'
)

start_ns="$(date +'%s%N')"
{

cat << EOF
struct entry { char* key; uint32_t value; };
%%
EOF

for (( i = 0; ; i++ )); do
  read -r key || break
  printf "%s\t%s\n" "${key}" "${i}"
done < "${1}"

} | "${gperf_cmd[@]}" > "${project}/hasher.c"
end_ns="$(date +'%s%N')"

python -c "print('gperf took', (${end_ns} - ${start_ns}) // 1000000, 'ms')" >&2

cat >> "${project}/hasher.c" << EOF
uint32_t lookup(const char *key, size_t len) {
    const struct entry* entry = in_word_set(key, len);
    return entry == NULL ? ((uint32_t) -1) : entry->value;
}
EOF

hash_table_size=$(grep -Po '(?<=#define MAX_HASH_VALUE )[0-9]+' "${project}/hasher.c")
(( hash_table_size++ ))
echo "${hash_table_size}" > "hash_table_size"

data_bytes_char=$(
  tr '\n' '\r' < "${project}/hasher.c" | \
  grep -Eo 'static const unsigned char asso_values\[\][^;]+;' | \
  tr -dc ',;' | \
  wc -c
)
data_bytes_short=$(
  tr '\n' '\r' < "${project}/hasher.c" | \
  grep -Eo 'static const unsigned short asso_values\[\][^;]+;' | \
  tr -dc ',;' | \
  wc -c
)
echo "$(( data_bytes_char + 2 * data_bytes_short ))" > "data_bytes"

GCC_FLAGS='-Wno-missing-field-initializers -Wno-unused-parameter'
(cd "${project}" && GCC_FLAGS="${GCC_FLAGS}" make)

mv "${project}/run" run
#rm -r "${project}"
