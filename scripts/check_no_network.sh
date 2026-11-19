#!/bin/sh
set -eu

binary=${1:-target/release/universal-transliterator}
output=${2:-build/network.trace}

if [ ! -x "$binary" ]; then
    echo "not executable: $binary" >&2
    exit 2
fi
mkdir -p -- "$(dirname -- "$output")"
command -v cc >/dev/null 2>&1 || {
    echo "missing C compiler required for the seccomp network test" >&2
    exit 2
}

harness=$(dirname -- "$output")/no-internet-exec
cc -std=c11 -O2 -Wall -Wextra -Werror \
    scripts/no_internet_exec.c -o "$harness"

"$harness" "$binary" --self-test >"$output" 2>&1 || {
    cat "$output" >&2
    echo "FAIL: offline self-test failed under the Internet-socket filter" >&2
    exit 1
}
cat "$output"
