#!/bin/bash

run_query() {
    echo "Running query: $1"
    cargo run -q -- "$1"
}

cargo build
run_query "CREATE PROJECT test WITH NAME 'Test Project'"
run_query "SELECT * FROM projects"
run_query "CREATE ISSUE IN test WITH TITLE 'Something is wrong'"
run_query "SELECT * FROM issues"
run_query "UPDATE ISSUE test#1 SET description = 'This is a test issue'"
run_query "SELECT * FROM issues"
run_query "COMMENT ON ISSUE test#1 WITH 'Some Comment'"
run_query "SELECT * FROM comments WHERE issue = 'test#1'"