#!/usr/bin/bash

set -eux

# Install clippy and rustfmt
rustup_tools() {
    rustup component add clippy rustfmt
}

# Remove old builds from cache
clean() {
    find target -type f -name "mathematica-notebook-filter" -exec rm '{}' +
    find target -type f -name "mathematica-notebook-filter-*" -exec rm '{}' +
}


main() {
    rustup_tools
    clean
}

main
