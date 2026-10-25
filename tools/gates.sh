#!/usr/bin/env bash
# The four gates, as one line of output.
#
#   tools/gates.sh
#
# `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
# `cargo test` and `cargo check --no-default-features`, in that order, stopping at
# the first failure. Everything each one prints is captured; what comes out is
# either one summary line or the first twenty lines of the failure. The point is
# the token cost of pasting a test run into a conversation, not the runs
# themselves — the gates are still all four, still every time.
set -o pipefail

cd "$(dirname "$0")/.." || exit 2

report() {
    echo "GATES FAILED: $1"
    printf '%s\n' "$2" | head -20
    exit 1
}

out=$(cargo fmt --check 2>&1) || report "cargo fmt --check" "$out"
out=$(cargo clippy --offline --all-targets -- -D warnings 2>&1) || report "cargo clippy" "$out"
tests=$(cargo test --offline 2>&1) || report "cargo test" "$tests"
out=$(cargo check --offline --no-default-features 2>&1) || report "cargo check --no-default-features" "$out"

echo "GATES OK — $(printf '%s\n' "$tests" | grep -E '^test result' | tail -1)"
