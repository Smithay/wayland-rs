#!/bin/bash

# early exit on any failure
set -e

# First install kcov

KCOV_VERSION=v36
KCOV_INSTALL=$HOME/kcov

mkdir -p $KCOV_INSTALL

wget https://github.com/SimonKagstrom/kcov/archive/$KCOV_VERSION.tar.gz
tar xzf $KCOV_VERSION.tar.gz
cd kcov-*
mkdir build
cd build
cmake .. -DCMAKE_INSTALL_PREFIX=$KCOV_INSTALL
make
make install
cd ../..

# Then run the coverage

export RUSTFLAGS=" -C relocation-model=dynamic-no-pic -C link-dead-code -C opt-level=0 -C debuginfo=2 -C link-args=-Wl,--no-gc-sections "
cargo clean

function run_tests {
    for file in target/debug/*;
    do
        [[ -f "${file}" && -x "${file}" ]] || continue;
        mkdir -p "target/cov/$(basename $file)"
        $KCOV_INSTALL/bin/kcov --exclude-pattern=/.cargo,/usr/lib,/tests --verify "target/cov/$(basename $file)" "${file}"
    done
}

cargo test --no-run --all --features ""
run_tests
bash <(curl -s https://codecov.io/bash) -cF full_rust
rm -rf target/cov/
find target/debug -maxdepth 1 -type f -delete

cargo test --no-run --all --features "client_native"
run_tests
bash <(curl -s https://codecov.io/bash) -cF client_native
rm -rf target/cov/
find target/debug -maxdepth 1 -type f -delete

cargo test --no-run --all --features "server_native"
run_tests
bash <(curl -s https://codecov.io/bash) -cF server_native
rm -rf target/cov/
find target/debug -maxdepth 1 -type f -delete

cargo test --no-run --all --features "client_native server_native"
run_tests
bash <(curl -s https://codecov.io/bash) -cF both_native
rm -rf target/cov/
find target/debug -maxdepth 1 -type f -delete
