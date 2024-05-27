#!/bin/bash

set -eu

project='rs_template'
cp -a "$(dirname "${0}")/${project}" .

{

cat << EOF
static ENTRIES: &'static [(&'static str, u32)] = &[
EOF

for (( i = 0; ; i++ )); do
  read -r key || break
  cat << EOF
    ("${key}", ${i}),
EOF
done < "${1}"

cat << EOF
];

pub struct Hasher();

impl Hasher {
    pub fn new() -> Self {
        Self()
    }

    pub fn lookup(&self, key: &str) -> u32 {
        for &(k, v) in ENTRIES {
            if k == key {
                return v
            }
        }
        0
    }
}
EOF

} > "${project}/src/hasher.rs"

(cd "${project}" && cargo build --release)

mv "${project}/target/release/${project}" run
#rm -r "${project}"
