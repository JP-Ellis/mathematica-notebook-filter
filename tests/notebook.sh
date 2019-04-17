#!/bin/bash

# Echo all commands before executing them
set -o xtrace
# Forbid any unset variables
set -o nounset
# Exit on any error
set -o errexit

TESTS=()
KCOV_BIN="./target/kcov-master/build/src/kcov"
RS_BIN=$(cargo run 2>&1 | grep 'Running' | sed 's/.*`\(.*\)`/\1/')
INPUT_NOTEBOOK="tests/notebook.nb"

TESTS+=("valid_notebook")
valid_notebook() {
    local out_dir="target/kcov/${FUNCNAME[0]}"
    mkdir -p $out_dir
    $KCOV_BIN \
        --verify \
        $out_dir \
        $RS_BIN -i $INPUT_NOTEBOOK -o tests/out-notebook.nb
}

TESTS+=("valid_pipe")
valid_pipe() {
    local out_dir="target/kcov/${FUNCNAME[0]}"
    mkdir -p $out_dir
    $KCOV_BIN \
        --verify \
        $out_dir \
        $RS_BIN <$INPUT_NOTEBOOK >tests/out-pipe.nb
}

TESTS+=("valid_notebook_v")
valid_notebook_v() {
    local out_dir="target/kcov/${FUNCNAME[0]}"
    mkdir -p $out_dir
    $KCOV_BIN \
        --verify \
        $out_dir \
        $RS_BIN -i $INPUT_NOTEBOOK -o tests/out-v.nb
}

TESTS+=("valid_notebook_vv")
valid_notebook_vv() {
    local out_dir="target/kcov/${FUNCNAME[0]}"
    mkdir -p $out_dir
    $KCOV_BIN \
        --verify \
        $out_dir \
        $RS_BIN -i $INPUT_NOTEBOOK -o tests/out-vv.nb
}

TESTS+=("valid_notebook_vvv")
valid_notebook_vvv() {
    local out_dir="target/kcov/${FUNCNAME[0]}"
    mkdir -p $out_dir
    $KCOV_BIN \
        --verify \
        $out_dir \
        $RS_BIN -i $INPUT_NOTEBOOK -o tests/out-vvv.nb
}

TESTS+=("invalid_argument")
invalid_argument() {
    local out_dir="target/kcov/${FUNCNAME[0]}"
    mkdir -p $out_dir
    $KCOV_BIN \
        --verify \
        $out_dir \
        $RS_BIN --foobar \
        || true
}

TESTS+=("inexistent_notebook")
inexistent_notebook() {
    local out_dir="target/kcov/${FUNCNAME[0]}"
    mkdir -p $out_dir
    $KCOV_BIN \
        --verify \
        $out_dir \
        $RS_BIN -i tests/not-a-notebook.nb -o tests/out-inexistent.nb \
        || true
}

TESTS+=("not_notebook")
not_notebook() {
    local out_dir="target/kcov/${FUNCNAME[0]}"
    mkdir -p $out_dir
    $KCOV_BIN \
        --verify \
        $out_dir \
        $RS_BIN -i Cargo.toml -o tests/out-not.nb \
        || true
}

main() {
    for t in ${TESTS[@]} ; do
        $t
    done

    if [[ -d target/kcov/kcov-merged ]]; then
       mv target/kcov/kcov-merged target/kcov/kcov-cargo
       $KCOV_BIN --coveralls-id $TRAVIS_JOB_ID --merge target/kcov/kcov-merged target/kcov/kcov-cargo ${TESTS[@]/#/target/kcov/}
       rm -rf target/kcov/kcov-cargo
    else
        $KCOV_BIN --coveralls-id $TRAVIS_JOB_ID --merge target/kcov/kcov-merged ${TESTS[@]/#/target/kcov/}
    fi

    rm -rf ${TESTS[@]/#/target/kcov/}
}

main
