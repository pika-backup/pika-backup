#!/bin/sh

git ls-files \
	"pika-backup/src/*.ui" "pika-backup/src/*.rs" "pika-backup-common/src/*.rs" "pika-backup-monitor/src/*.rs" "data/*.ui" "data/*.desktop.in" "data/*.xml.in" \
	> po/POTFILES.in

cd po
intltool-update --maintain 2> /dev/null
cat missing | grep '^\(pika-backup/src\|data\)/'
code=$?
rm missing

if [ $code -eq 0 ]
then
	exit 1
fi
