#!/bin/bash -xe

APP_ID=$(cat data/APPLICATION_ID).Devel
REPO_DIR=flatpak_repo

flatpak-builder \
  --user --verbose --force-clean -y --repo=$REPO_DIR flatpak_out build-aux/$APP_ID.json
flatpak build-bundle $REPO_DIR $APP_ID.flatpak $APP_ID
flatpak --user install -y $APP_ID.flatpak
flatpak run $APP_ID//master
