#!/bin/sh

./flatpak-builder-tools/cargo/flatpak-cargo-generator.py -o build-aux/generated-sources.json Cargo.lock
