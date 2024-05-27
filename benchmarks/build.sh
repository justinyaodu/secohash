#!/bin/bash

set -eu
cd "$(dirname "${0}")"

(cd .. && cargo build)
(cd utils && make)

bin='bin'
rm -rf "${bin}"
mkdir "${bin}"

for dataset in datasets/*.txt; do
  dataset="$(realpath "${dataset}")"
  dataset_name="$(basename "${dataset}" .txt)"

  for impl in impls/*.sh; do
    impl="$(realpath "${impl}")"
    impl_name="$(basename "${impl}" .sh)"

    bench_dir="${bin}/${dataset_name}__${impl_name}"
    mkdir "${bench_dir}"

    tput setaf 6
    echo -e "\n######## ${bench_dir} ########\n"
    tput sgr0

    (cd "${bench_dir}" && bash "${impl}" "${dataset}")

    if ! [ -f "${bench_dir}/run" ]; then
      tput setaf 1
      echo "build failed: ${bench_dir}"
      tput sgr0
    fi
  done
done
