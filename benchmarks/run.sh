#!/bin/bash

set -eu
cd "$(dirname "${0}")"

n="${1}"
dataset_filter="${2:-}"
impl_filter="${3:-}"

bin='bin'
results='results'
rm -rf "${results}"
mkdir "${results}"

for dataset in datasets/*.txt; do
  dataset_name="$(basename "${dataset}" .txt)"

  grep -Eq "${dataset_filter}" <<< "${dataset_name}" || continue

  shuffled="${results}/${dataset_name}_${n}.txt"
  hyperfine_cmd=(
    hyperfine
    --shell none
    --warmup 1
    --input "${shuffled}"
    --export-json "${results}/${dataset_name}.json"
  )

  impl_count=0
  for impl in impls/*.sh; do
    #grep -q 'control' <<< "${impl}" && continue
    grep -Eq "${impl_filter}" <<< "${impl}" || continue
    impl_name="$(basename "${impl}" .sh)"
    bench_run="${bin}/${dataset_name}__${impl_name}/run"
    [ -f "${bench_run}" ] || continue
    hyperfine_cmd+=("${bench_run}")
    (( ++impl_count ))
  done
  (( impl_count )) || continue

  utils/bin/shuffler "${n}" < "${dataset}" > "${shuffled}"

  "${hyperfine_cmd[@]}"

  rm "${shuffled}"
done
