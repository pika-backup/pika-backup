#!/bin/sh -x

MESON_SOURCE_ROOT="$1"
CARGO_TARGET_DIR="$2"
CARGO_OPTIONS="$3"
CARGO_OUTPUT="$4"
OUTPUT="$5"

cargo build \
    --manifest-path "${MESON_SOURCE_ROOT}/Cargo.toml" \
    --target-dir "$CARGO_TARGET_DIR" \
    $CARGO_OPTIONS && \
cp "$CARGO_OUTPUT" "$OUTPUT"
