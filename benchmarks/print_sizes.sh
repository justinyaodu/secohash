#!/bin/bash

set -eu
cd "$(dirname "${0}")"

bin='bin'

for dataset in datasets/*.txt; do
  dataset_name="$(basename "${dataset}" .txt)"

  tput setaf 6
  echo -e "\n######## ${dataset_name} ########\n"
  tput sgr0

  echo "hash_table_size:"
  c_array_hash_table_size="$(cat "${bin}/${dataset_name}__c_array/hash_table_size")"
  echo -e "c_array\t${c_array_hash_table_size}"

  for impl_name in c_gperf c_seco; do
    file="${bin}/${dataset_name}__${impl_name}/hash_table_size"
    [ -f "${file}" ] || continue
    hash_table_size="$(cat "${file}")"
    ratio="$(python -c "print(f'{${hash_table_size} / ${c_array_hash_table_size}:.3}')")"
    echo -e "${impl_name}\t${hash_table_size} (${ratio}x)"
  done

  echo
  echo "data_bytes:"
  for impl_name in c_gperf c_seco; do
    file="${bin}/${dataset_name}__${impl_name}/data_bytes"
    [ -f "${file}" ] || continue
    data_bytes="$(cat "${file}")"
    echo -e "${impl_name}\t${data_bytes}"
  done
done
