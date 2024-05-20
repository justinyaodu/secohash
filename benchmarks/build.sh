#!/bin/bash

set -eu
cd "$(dirname "${0}")"

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

    (cd "${bench_dir}" && bash "${impl}" "${dataset}")
  done
done
