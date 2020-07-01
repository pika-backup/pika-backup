#!/bin/sh

sed '0,/share=network/{//d;}' ci.manifest.yml\
| sed '0,/build-args/{//d;}' \
| sed 's/type: dir/type: archive/' \
| sed 's/path: ..\//url: \n         sha256: /' \
> flathub.manifest.yml
