#!/bin/sh

git ls-files \
	"pika-backup/*.ui" "pika-backup/*.rs" "pika-backup-common/*.rs" "pika-backup-monitor/*.rs" "data/*.ui" "data/*.desktop.in" "data/*.xml.in" \
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
