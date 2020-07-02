#!/bin/sh

sed '0,/share=network/{//d;}' ci.manifest.yml\
| sed '0,/build-args/{//d;}' \
| sed 's/-Dprofile=dev/-Dprofile=release/' \
| sed 's/type: git/type: archive/' \
| sed 's/path: ..\//url: \n        sha256: /' \
> flathub.manifest.yml
