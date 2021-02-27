#!/bin/sh

src="$(find src/ -path '*.rs')"
git ls-files \
	$src "data/*.ui" "data/*.desktop.in" "data/*.xml.in" \
	> po/POTFILES.in

cd po
intltool-update --maintain 2> /dev/null
cat missing | grep '^\(src\|data\)/'
code=$?
rm missing

if [ $code -eq 0 ]
then
	exit 1
fi
