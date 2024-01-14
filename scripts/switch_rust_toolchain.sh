#!/bin/sh

set -eu

RUST_TOOLCHAIN=

SCRIPTS_DIR=$(dirname "$0")
BASENAME=$(basename "$0")
WORKSPACE_DIR=$(cd $SCRIPTS_DIR/.. && pwd)
RUST_TOOLCHAIN_FILE="$WORKSPACE_DIR/rust-toolchain"

MSRV=$(cat "$WORKSPACE_DIR/truc/Cargo.toml" | sed -n -e 's/^\s*rust-version\s*=\s*"\([^"]*\)"\s*$/\1/p')

usage() {
    echo "$BASENAME -c|--clear"
    echo "    clear \`rust-toolchain\` (use system default)"
    echo "$BASENAME -m|--msrv"
    echo "    select \"$MSRV\""
    echo "$BASENAME -s|--stable"
    echo "    select \"stable\""
    echo "$BASENAME -n|--nightly"
    echo "    select \"nightly\""
    echo "$BASENAME -v|--version <version>"
    echo "    select specific version"
    echo "$BASENAME -h|--help"
    echo "    show this help and exit"
}

while [ $# -gt 0 ]; do
    case $1 in
        -c|--clear)
            RUST_TOOLCHAIN=
            shift
            ;;
        -m|--msrv)
            RUST_TOOLCHAIN="$MSRV"
            shift
            ;;
        -s|--stable)
            RUST_TOOLCHAIN="stable"
            shift
            ;;
        -n|--nightly)
            RUST_TOOLCHAIN="nightly"
            shift
            ;;
        -v|--version)
            RUST_TOOLCHAIN="$2"
            shift
            shift
            ;;
        -h|--help)
            usage
            exit 0
            shift
            ;;
        -*|--*)
            echo "Unknown option $1" > /dev/stderr
            echo > /dev/stderr
            usage > /dev/stderr
            exit 1
            ;;
        *)
            echo "Unexpected positional argument" > /dev/stderr
            echo > /dev/stderr
            usage > /dev/stderr
            exit 1
            ;;
    esac
done

echo "Removing \`Cargo.lock\`..."
rm -f "$WORKSPACE_DIR/Cargo.lock"

if [ x"$RUST_TOOLCHAIN" = x ]
then
    echo "Clearing \`$RUST_TOOLCHAIN_FILE\`..."
    rm -f "$RUST_TOOLCHAIN_FILE"
else
    echo "Switching $RUST_TOOLCHAIN_FILE to \"$RUST_TOOLCHAIN\""
    echo "$RUST_TOOLCHAIN" >| "$RUST_TOOLCHAIN_FILE"
fi 
