#!/bin/sh

set -eu

BASENAME=$(basename "$0")

MODE=
FILTERS=

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
            FILTERS="$FILTERS $1"
            shift
            ;;
    esac
done

MODE="${MODE:-TEST}"

PACKAGES="$(ls asm_tests)"

RESULT=0

for PACKAGE in $PACKAGES
do
    PACKAGE_PATH="asm_tests/$PACKAGE"
    TESTS="$(sort -u < "asm_tests/$PACKAGE/tests.txt")"
    if [ -n "$FILTERS" ]
    then
        TESTS=$(
            (
                for t in $TESTS
                do
                    for f in $FILTERS
                    do
                        echo "$t" | grep -F "$f"
                    done
                done
            ) | sort -u
        )
    fi
    case "$MODE" in
        TEST)
            for t in $TESTS
            do
                echo "$MODE: $PACKAGE / $t"
                cargo asm -p "$PACKAGE" --features "$t" --bin "$PACKAGE" "$t" --asm >| "$PACKAGE_PATH/$t.asm"
                if diff -u "$PACKAGE_PATH/$t.asm.snap" "$PACKAGE_PATH/$t.asm"
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
                cargo asm -p "$PACKAGE" --features "$t" --bin "$PACKAGE" "$t" --asm >| "$PACKAGE_PATH/$t.asm.snap"
            done
            ;;
        *)
            echo "Unexpected mode $MODE" > /dev/stderr
            exit 2
            ;;
    esac
done

exit "$RESULT"
