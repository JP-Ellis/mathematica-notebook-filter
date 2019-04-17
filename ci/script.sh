#!/usr/bin/bash

# Echo all commands before executing them
set -o xtrace
# Forbid any unset variables
set -o nounset
# Exit on any error
set -o errexit

# Ensure there are no outstanding lints.
check_lints() {
    cargo clippy $FEATURES
}

# Ensure the code is correctly formatted.
check_format() {
    cargo fmt -- --check
}

# Run the test suite.
check_tests() {
    cargo test $FEATURES
}

check_command_line() {
    cargo run -- -vvv -i tests/notebook.nb -o tests/notebook.min.nb
    if [[ $(wc -c < tests/notebook.nb) -le $(wc -c < tests/notebook.min.nb) ]]; then
        echo "No reduction in file size ($(wc -c < tests/notebook.nb) => $(wc -c < tests/notebook.min.nb))." >&2
        false
    fi
}

main() {
    check_lints
    check_format
    check_tests
    check_command_line
}

main
