#!/bin/sh -e

cd "$(dirname "$0")/.."

yq -o json '
    .["id"] += ".Devel" |
    .["desktop-file-name-suffix"] = " 🚧" |
    .["sdk-extensions"] += "org.freedesktop.Sdk.Extension.llvm16" |
    .["build-options"] += {"build-args": ["--share=network"]} |
    .["build-options"]["append-path"] += ":/usr/lib/sdk/llvm16/bin" |
    .["build-options"] += {"env": {
      "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER" : "clang",
      "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER" : "clang",
      "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS" : "-C link-arg=-fuse-ld=/usr/lib/sdk/rust-stable/bin/mold",
      "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS" : "-C link-arg=-fuse-ld=/usr/lib/sdk/rust-stable/bin/mold"
    }} |
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
