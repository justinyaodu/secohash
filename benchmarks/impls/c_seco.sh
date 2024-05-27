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

(cd "${project}" && make)

mv "${project}/run" run
#rm -r "${project}"
