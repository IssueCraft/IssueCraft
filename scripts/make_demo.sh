#!/bin/bash

set -eu

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
pushd "$script_dir/.." >/dev/null || exit 1

cargo build --release

PATH="$(pwd)/target/release:$PATH"
export PATH

mkdir -p ./tmp
pushd ./tmp >/dev/null || exit 1

export ISSUECRAFT_DB="./demo.redb"

if [[ -f "$ISSUECRAFT_DB" ]]; then
    rm "$ISSUECRAFT_DB"
fi

vhs "$script_dir/demo.tape" -o "$script_dir/../assets/demo.gif"

if [[ -f "$ISSUECRAFT_DB" ]]; then
    rm "$ISSUECRAFT_DB"
fi

popd >/dev/null || exit 1