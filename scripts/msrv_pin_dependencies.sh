#!/bin/sh

set -eu

set -x
cargo update -p ppv-lite86 --precise 0.2.17
cargo update -p derive_more --precise 0.99.17

