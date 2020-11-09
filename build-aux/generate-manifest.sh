#!/bin/sh


cd "$(dirname "$0")/.."

yq '
    .modules[-1].sources[0] = {type: "dir", path: ".."} |
    .modules[-1]["config-opts"] = ["-Dprofile=dev"]' \
    build-aux/org.gnome.World.PikaBackup.yml > build-aux/org.gnome.World.PikaBackup.Devel.json

./build-aux/flatpak-cargo-generator.py \
    -o build-aux/generated-sources.json Cargo.lock
