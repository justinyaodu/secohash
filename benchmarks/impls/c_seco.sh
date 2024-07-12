#!/bin/bash

set -eu

project='c_template'
cp -a "$(dirname "${0}")/${project}" .

seco_cmd=(
  "$(dirname "${0}")/../../target/debug/secohash"
)

if ! "${seco_cmd[@]}" < "${1}" > "${project}/hasher.c"; then
  exit 0
fi

hash_table_size="$(
  tr '\n' '\r' < "${project}/hasher.c" | \
  grep -Eo 'const struct entry entries\[\] = [^;]+;' | \
  tr '\r' '\n' | \
  wc -l)"
(( hash_table_size -= 2 ))
echo "${hash_table_size}" > "hash_table_size"

data_bytes=$(
  grep -Eo 'static const uint8_t t[0-9]+\[\] = [^;]+;' "${project}/hasher.c" | \
  tr -dc ',;' | \
  wc -c
)
echo "${data_bytes}" > "data_bytes"

(cd "${project}" && GCC_FLAGS='-Werror' make)

mv "${project}/run" run
#rm -r "${project}"
