#!/bin/sh

set -eu

set -x

cargo update -p ppv-lite86 --precise 0.2.17
cargo update -p derive_more --precise 0.99.17
cargo update -p either --precise 1.13.0
cargo update -p pretty_assertions --precise 1.3.0
cargo update -p libc --precise 0.2.163
cargo update -p libm --precise 0.2.9
cargo update -p os_str_bytes --precise 6.1.0
cargo update -p textwrap --precise 0.16.1
cargo update -p quote --precise 1.0.40
cargo update -p proc-macro2 --precise 1.0.101
cargo update -p ryu --precise 1.0.20
cargo update -p itoa --precise 1.0.15

cd examples/readme

cargo update -p derive_more --precise 0.99.17
cargo update -p either --precise 1.13.0
cargo update -p quote --precise 1.0.40
cargo update -p proc-macro2 --precise 1.0.101
cargo update -p ryu --precise 1.0.20
cargo update -p itoa --precise 1.0.15

cd -

