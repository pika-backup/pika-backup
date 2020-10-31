#!/bin/sh -x

eval $*

cargo build \
    --manifest-path "${MESON_SOURCE_ROOT}/Cargo.toml" \
    --target-dir "$CARGO_TARGET_DIR" \
    $CARGO_OPTIONS && \
cp "$CARGO_OUTPUT" "$OUTPUT"
