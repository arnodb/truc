export RUST_BACKTRACE := "1"

# Build

build:
    cargo build --all-features --all-targets

watch_build:
    cargo watch -x "build --all-features --all-targets"

clippy:
    cargo clippy --all-features --all-targets -- -D warnings

watch_clippy:
    cargo watch -x "clippy --all-features --all-targets -- -D warnings"

test *args:
    cargo test --all-features {{args}}

test_msrv:
    cargo test --features msrv

asm_tests:
    ./scripts/asm_test.sh

check_all:
    just stable
    cargo clippy --all-features --all-targets -- -D warnings
    cargo build --all-features
    cargo test --all-features

    just msrv
    cargo build --features msrv
    cargo test --features msrv

    just nightly
    cargo build --all-features
    cargo test --all-features

    just stable
    ./scripts/asm_test.sh

# Toolchain management

stable:
    ./scripts/switch_rust_toolchain.sh -c

nightly:
    ./scripts/switch_rust_toolchain.sh -n

msrv:
    ./scripts/switch_rust_toolchain.sh -m

# Formatting

fmt:
    cargo fmt

fmt_nightly:
    just nightly
    cargo fmt

# Examples

run_example example *args:
    cargo run -p $(basename {{example}}) {{args}}

