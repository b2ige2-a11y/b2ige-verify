#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
    echo "usage: $0 OUTPUT.typescript" >&2
    exit 64
fi

B2IGE_RECORD_REPO=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
B2IGE_RECORD_OUTPUT=$1
case "$B2IGE_RECORD_OUTPUT" in
    /*) ;;
    *) B2IGE_RECORD_OUTPUT="$PWD/$B2IGE_RECORD_OUTPUT" ;;
esac

if [ -e "$B2IGE_RECORD_OUTPUT" ]; then
    echo "refusing to overwrite existing recording: $B2IGE_RECORD_OUTPUT" >&2
    exit 64
fi
mkdir -p "$(dirname "$B2IGE_RECORD_OUTPUT")"

if [ -n "${B2IGE_BIN_DIR:-}" ]; then
    B2IGE_RECORD_BIN_DIR=$B2IGE_BIN_DIR
elif [ -x "$B2IGE_RECORD_REPO/bin/b2ige" ]; then
    B2IGE_RECORD_BIN_DIR="$B2IGE_RECORD_REPO/bin"
else
    B2IGE_RECORD_BIN_DIR="$B2IGE_RECORD_REPO/target/release"
fi
case "$B2IGE_RECORD_BIN_DIR" in
    /*) ;;
    *) B2IGE_RECORD_BIN_DIR="$PWD/$B2IGE_RECORD_BIN_DIR" ;;
esac
export B2IGE_BIN_DIR="$B2IGE_RECORD_BIN_DIR"

cd "$B2IGE_RECORD_REPO"
case "$(uname -s)" in
    Darwin)
        exec script -q "$B2IGE_RECORD_OUTPUT" "$B2IGE_RECORD_REPO/scripts/demo-blindtest.sh"
        ;;
    *)
        exec script -q -c "$B2IGE_RECORD_REPO/scripts/demo-blindtest.sh" "$B2IGE_RECORD_OUTPUT"
        ;;
esac
