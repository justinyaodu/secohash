#!/bin/bash

set -eu

project='rs_template'
cp -a "$(dirname "${0}")/${project}" .

(cd "${project}" && cargo build --release)

mv "${project}/target/release/${project}" run
#rm -r "${project}"
