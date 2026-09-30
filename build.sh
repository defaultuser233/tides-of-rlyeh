#!/usr/bin/env bash
# Native Linux build. Run this ON Linux.
#
#   ./build.sh
#
# Produces: target/release/rust-fps
#
# The binary links against the system glibc and loads X11/GL at runtime, so the
# usual desktop packages must be present:
#   Debian/Ubuntu : sudo apt install libx11-6 libgl1 libasound2
#   Fedora        : sudo dnf install libX11 mesa-libGL alsa-lib

set -euo pipefail
cd "$(dirname "$0")"

echo "==> Building for Linux (native)"
cargo build --release

bin="target/release/rust-fps"
if [[ ! -f "$bin" ]]; then
    echo "Build finished but $bin is missing" >&2
    exit 1
fi

echo "==> Built: $bin  ($(du -h "$bin" | cut -f1))"
file "$bin" 2>/dev/null || true
