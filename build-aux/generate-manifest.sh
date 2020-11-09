#!/bin/sh


cd "$(dirname "$0")/.."

yq '
    .["app-id"] += ".Devel" |
    .["desktop-file-name-suffix"] = " ☢" |
    .modules[-1].sources[0] = {type: "dir", path: ".."} |
    .modules[-1]["config-opts"] = ["-Dprofile=dev", "-Dapp_id_suffix=.Devel"]' \
    build-aux/org.gnome.World.PikaBackup.yml > build-aux/org.gnome.World.PikaBackup.Devel.json

./build-aux/flatpak-cargo-generator.py \
    -o build-aux/generated-sources.json Cargo.lock
