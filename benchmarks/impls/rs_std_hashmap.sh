#!/bin/bash

set -eu

project='rs_template'
cp -a "$(dirname "${0}")/${project}" .

{

cat << EOF
use std::collections::HashMap;

pub struct Hasher(HashMap<&'static str, u64>);

impl Hasher {
    pub fn new() -> Self {
        Self(HashMap::from([
EOF

for (( i = 1; ; i++ )); do
  read -r key || break
  cat << EOF
            ("${key}", ${i}),
EOF
done < "${1}"

cat << EOF
        ]))
    }

    pub fn lookup(&self, key: &str) -> u64 {
        self.0.get(key).cloned().unwrap_or(0)
    }
}
EOF

} > "${project}/src/hasher.rs"

(cd "${project}" && cargo build --release)

mv "${project}/target/release/${project}" run
#rm -r "${project}"
