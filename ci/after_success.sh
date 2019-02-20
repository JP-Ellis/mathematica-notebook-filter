#!/usr/bin/bash

set -eux

install_kcov() {
    set -e
    # Download and install kcov
    wget https://github.com/SimonKagstrom/kcov/archive/master.tar.gz -O - | tar -xz
    cd kcov-master
    mkdir build
    cd build
    cmake ..
    make -j$(nproc)
    make install DESTDIR=../../kcov-build
    cd ../..
    rm -rf kcov-master
    set +e
}

run_kcov() {
    # Run kcov on all the test suites
    for file in target/debug/mathematica_notebook_filter-*[^\.d]; do
        mkdir -p "target/cov/$(basename $file)";
        echo "Testing $(basename $file)"
        ./kcov-build/usr/local/bin/kcov \
            --exclude-pattern=/.cargo,/usr/lib\
            --verify "target/cov/$(basename $file)" \
            "$file";
    done

    # Run kcov with the binary and test various sets of arguments.
    executable="target/debug/mathematica-notebook-filter"

    mkdir -p "target/cov/exe-valid1"
    ./kcov-build/usr/local/bin/kcov \
        --exclude-pattern=/.cargo,/usr/lib \
        --verify "target/cov/exe-valid1" \
        "$executable" -i "ci/test_notebook.nb" -o "ci/test_notebook_min.nb"

    mkdir -p "target/cov/exe-valid2"
    ./kcov-build/usr/local/bin/kcov \
        --exclude-pattern=/.cargo,/usr/lib \
        --verify "target/cov/exe-valid2" \
        "$executable" -v <"ci/test_notebook.nb" >"ci/test_notebook_min_pipe.nb"

    mkdir -p "target/cov/exe-valid3"
    ./kcov-build/usr/local/bin/kcov \
        --exclude-pattern=/.cargo,/usr/lib \
        --verify "target/cov/exe-valid3" \
        "$executable" -vv -i "ci/test_notebook.nb" -o "ci/test_notebook.nb"

    # Also test a few invalid arguments
    mkdir -p "target/cov/exe-invalid1"
    ./kcov-build/usr/local/bin/kcov \
        --exclude-pattern=/.cargo,/usr/lib \
        --verify "target/cov/exe-invalid1" \
        "$executable" -vvv -i "ci/invalid.nb" -o "ci/new.nb"

    mkdir -p "target/cov/exe-invalid2"
    ./kcov-build/usr/local/bin/kcov \
        --exclude-pattern=/.cargo,/usr/lib \
        --verify "target/cov/exe-invalid2" \
        "$executable" -vvv --foobar

    mkdir -p "target/cov/exe-invalid3"
    ./kcov-build/usr/local/bin/kcov \
        --exclude-pattern=/.cargo,/usr/lib \
        --verify "target/cov/exe-invalid3" \
        "$executable" -vvv <"ci/script.sh" >"/dev/null"

    bash <(curl -s https://codecov.io/bash)
    echo "Uploaded code coverage"
}

kcov_suite() {
    if [[ "$TRAVIS_RUST_VERSION" == "stable" ]]; then
        install_kcov
        run_kcov
    fi
}

main() {
    kcov_suite
}

main
