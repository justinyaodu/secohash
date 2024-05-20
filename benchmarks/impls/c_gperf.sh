#!/bin/bash

set -eu

project='c_template'
cp -a "$(dirname "${0}")/${project}" .

gperf_cmd=(
  gperf
  --language=ANSI-C
  --struct-type
  --readonly-tables
  --lookup-function-name=lookup
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

(cd "${project}" && GCC_FLAGS='-Wno-missing-field-initializers' make)

mv "${project}/run" run
#rm -r "${project}"
