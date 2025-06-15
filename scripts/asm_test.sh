#!/bin/sh

set -eu

BASENAME=$(basename "$0")

MODE=

usage() {
    echo "$BASENAME -s|--snapshot [FILTER ...]"
    echo "    regenerate the snapshot(s)"
    echo "$BASENAME [-t|--test] [FILTER ...]"
    echo "    run the test(s)"
    echo "$BASENAME -h|--help"
    echo "    show this help and exit"
}

while [ $# -gt 0 ]; do
    case $1 in
        -t|--test)
            if [ -n "$MODE" ]
            then
                echo "too many modes"
                exit 1
            fi
            MODE=TEST
            shift
            ;;
        -s|--snapshot)
            if [ -n "$MODE" ]
            then
                echo "too many modes"
                exit 1
            fi
            MODE=SNAPSHOT
            shift
            ;;
        -h|--help)
            usage
            exit 0
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

MODE="${MODE:-TEST}"

PACKAGES="$(ls asm_tests)"

RESULT=0

for PACKAGE in $PACKAGES
do
    PACKAGE_PATH="asm_tests/$PACKAGE"
    TESTS="$(cat "asm_tests/$PACKAGE/tests.txt")"
    case "$MODE" in
        TEST)
            for t in $TESTS
            do
                echo "$MODE: $PACKAGE / $t"
                cargo asm -p "$PACKAGE" --bin "$PACKAGE" "$t" --llvm >| "$PACKAGE_PATH/$t.llvm"
                if diff -u "$PACKAGE_PATH/$t.llvm.snap" "$PACKAGE_PATH/$t.llvm"
                then
                    echo "$PACKAGE / $t ok."
                else
                    echo "$PACKAGE / $t FAILED!!!"
                    RESULT=1
                fi
            done
            ;;
        SNAPSHOT)
            for t in $TESTS
            do
                echo "$MODE: $PACKAGE / $t"
                cargo asm -p "$PACKAGE" --bin "$PACKAGE" "$t" --llvm >| "$PACKAGE_PATH/$t.llvm.snap"
            done
            ;;
        *)
            echo "Unexpected mode $MODE" > /dev/stderr
            exit 2
            ;;
    esac
done

exit "$RESULT"
