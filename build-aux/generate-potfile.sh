#!/bin/sh

git ls-files \
	"src/ui/*.rs" "src/ui.rs" "data/*.ui" "data/*.desktop.in" "data/*.xml.in" | \
	grep -v builder.rs \
	> po/POTFILES.in

cd po
intltool-update --maintain
