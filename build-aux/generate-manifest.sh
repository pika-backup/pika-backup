#!/bin/bash -e

cd "$(dirname "$0")/.."

yq -o json '
    .["id"] += ".Devel" |
    .["desktop-file-name-suffix"] = " 🚧" |
    .["build-options"] += {"build-args": ["--share=network"]} |
    .["finish-args"] += "--env=G_MESSAGES_DEBUG=all" |
    .["finish-args"] += "--env=RUST_BACKTRACE=1" |
    .modules[-1].sources = [{"type": "dir", "path": ".."}] |
    .modules[-1]["config-opts"] = ["-Dprofile=dev", "-Dapp_id_suffix=.Devel"]' \
    build-aux/org.gnome.World.PikaBackup.yml > build-aux/org.gnome.World.PikaBackup.Devel.json.new

set +e

cmp build-aux/org.gnome.World.PikaBackup.Devel.json build-aux/org.gnome.World.PikaBackup.Devel.json.new > /dev/null

if [[ $? -eq 0 ]]; then
  rm build-aux/org.gnome.World.PikaBackup.Devel.json.new
else
  mv build-aux/org.gnome.World.PikaBackup.Devel.json.new build-aux/org.gnome.World.PikaBackup.Devel.json
fi
