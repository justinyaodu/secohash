#!/bin/bash

set -eu

project='cpp_template'
cp -a "$(dirname "${0}")/${project}" .

(cd "${project}" && make)

mv "${project}/run" run
#rm -r "${project}"
