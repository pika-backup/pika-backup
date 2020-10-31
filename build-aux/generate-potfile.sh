#!/bin/sh

git ls-files \
	"src/*.rs" "data/*.ui" "data/*.desktop" "data/*.xml" | \
	grep -v builder.rs \
	> po/POTFILES.in

cd po
intltool-update --maintain
