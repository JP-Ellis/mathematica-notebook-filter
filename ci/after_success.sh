#!/bin/bash

# Echo all commands before executing them
set -o xtrace
# Forbid any unset variables
set -o nounset
# Exit on any error
set -o errexit

COVERAGE_RUN=false

coverage() {
    if [[ "$TRAVIS_RUST_VERSION" == "stable" ]]; then
        cargo tarpaulin \
              --all --features "$FEATURES" \
              --out Xml \
              --ciserver travis-ci --coveralls $TRAVIS_JOB_ID

        bash <(curl -s https://codecov.io/bash)
    fi
}

main() {
    coverage
}

main
