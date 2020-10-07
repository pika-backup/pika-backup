#!/bin/sh

URL=https://gitlab.gnome.org/World/pika-backup.git
VERSION="$(build-aux/meson-cargo-manifest.py package version)"

cat build-aux/ci.manifest.yml\
| sed 's/-Dprofile=dev/-Dprofile=release/' \
| sed 's/type: dir/type: git/' \
| sed "s|path: ../|url: $URL\n        tag: v$VERSION|" \
> build-aux/org.gnome.World.PikaBackup.yml
