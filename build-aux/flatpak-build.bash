#!/bin/bash -x

APP_ID=$(cat data/APPLICATION_ID)
REPO_NAME=pika-pile-dev
REPO_DIR=flatpak_repo

flatpak remove $APP_ID -y
flatpak-builder --install-deps-from=https://dl.flathub.org/repo/ \
  --user --verbose --force-clean -y --repo=$REPO_DIR flatpak_out build-aux/$APP_ID.yml
flatpak build-bundle $REPO_DIR $APP_ID.flatpak $APP_ID
flatpak --force remote-delete $REPO_NAME
flatpak --user remote-add --no-gpg-verify $REPO_NAME $REPO_DIR
flatpak --user install -y $REPO_NAME $APP_ID
flatpak run $APP_ID
