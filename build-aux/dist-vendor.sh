#!/bin/sh -x

mkdir "${MESON_DIST_ROOT}/.cargo"
cp "${MESON_SOURCE_ROOT}/build-aux/cargo-config.toml" "${MESON_DIST_ROOT}/.cargo/config"
cargo vendor "${MESON_DIST_ROOT}/vendor"
