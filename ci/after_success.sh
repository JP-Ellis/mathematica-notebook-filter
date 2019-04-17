#!/usr/bin/bash

# Echo all commands before executing them
set -o xtrace
# Forbid any unset variables
set -o nounset
# Exit on any error
set -o errexit

COVERAGE_RUN=false

run_kcov() {
    # Run kcov on all the test suites
    if [[ $COVERAGE_RUN != "true" ]]; then
        cargo coveralls
        COVERAGE_RUN=true
    fi
}

coverage_codecov() {
    if [[ "$TRAVIS_RUST_VERSION" != "stable" ]]; then
        return
    fi

    run_kcov

    bash <(curl -s https://codecov.io/bash) -s target/kcov
    echo "Uploaded code coverage to codecov.io"
}

coverage_coveralls() {
    if [[ "$TRAVIS_RUST_VERSION" != "stable" ]]; then
        return
    fi

    run_kcov

    # Data is automatically uploaded by kcov
}

main() {
    coverage_coveralls
    coverage_codecov
}

main
