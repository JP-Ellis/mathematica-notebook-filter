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
    # Try it once using `cargo run`
    cargo run -- -vvv -i tests/test_notebook.nb -o tests/test_notebook_min.nb
    if [[ $(wc -c < tests/test_notebook.nb) -le $(wc -c < tests/test_notebook_min.nb) ]]; then
        echo "No reduction in file size ($(wc -c < tests/test_notebook.nb) => $(wc -c < tests/test_notebook_min.nb))." >&2
        false
    fi

    # Try also by calling it manually
    ./target/debug/mathematica-notebook-filter -vvv -i tests/test_notebook.nb -o tests/test_notebook_min.nb
    if [[ $(wc -c < tests/test_notebook.nb) -le $(wc -c < tests/test_notebook_min.nb) ]]; then
        echo "No reduction in file size ($(wc -c < tests/test_notebook.nb) => $(wc -c < tests/test_notebook_min.nb))." >&2
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
