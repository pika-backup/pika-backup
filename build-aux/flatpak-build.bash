#!/bin/bash -xe

APP_ID=$(cat data/APPLICATION_ID)
REPO_DIR=flatpak_repo

flatpak-builder --install-deps-from=flathub \
  --user --verbose --force-clean -y --repo=$REPO_DIR flatpak_out build-aux/org.gnome.World.PikaBackup.Devel.json
flatpak build-bundle $REPO_DIR $APP_ID.flatpak $APP_ID
flatpak --user install -y $APP_ID.flatpak
flatpak run $APP_ID//master
