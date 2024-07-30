#!/bin/bash

set -eu
cd "$(dirname "${0}")"

n="${1}"

bin='bin'
results='results'
rm -rf "${results}"
mkdir "${results}"

for dataset in datasets/*.txt; do
  dataset_name="$(basename "${dataset}" .txt)"
  shuffled="${results}/${dataset_name}_${n}.txt"
  utils/bin/shuffler "${n}" < "${dataset}" > "${shuffled}"

  for impl in impls/*.sh; do
    grep -q 'control' <<< "${impl}" && continue
    impl_name="$(basename "${impl}" .sh)"
    bench_run="${bin}/${dataset_name}__${impl_name}/run"
    [ -f "${bench_run}" ] || continue
    output="${results}/${dataset_name}__${impl_name}.out"
    #echo "${bench_run}"
    "${bench_run}" < "${shuffled}" > "${output}"
    #valgrind "${bench_run}" < "${shuffled}" > "${output}"
  done

  diff --unified --from-file "${results}/${dataset_name}__"*.out
  echo "OK ${dataset_name}"

  rm "${shuffled}"
done
