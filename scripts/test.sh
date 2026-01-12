#!/bin/bash

set -eu

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
pushd "$script_dir/.." >/dev/null || exit 1

mkdir -p ./tmp
pushd ./tmp >/dev/null || exit 1

export ISSUECRAFT_DB="./test.redb"

run_query() {
    echo "Running query: $1"
    cargo run -q -- "$1"
}

cargo build

if [[ -f "$ISSUECRAFT_DB" ]]; then
    rm "$ISSUECRAFT_DB"
fi

run_query "CREATE PROJECT test WITH NAME 'Test Project'"
run_query "SELECT * FROM projects"
run_query "CREATE ISSUE OF KIND bug IN test WITH TITLE 'Something is wrong'"
run_query "SELECT * FROM issues"
run_query "UPDATE ISSUE test#1 SET description = 'This is a test issue'"
run_query "SELECT * FROM issues"
run_query "COMMENT ON ISSUE test#1 WITH 'Some Comment'"
run_query "SELECT * FROM comments WHERE issue = 'test#1'"

if [[ -f "$ISSUECRAFT_DB" ]]; then
    rm "$ISSUECRAFT_DB"
fi

popd >/dev/null || exit 1 # tmp
popd >/dev/null || exit 1 # project root