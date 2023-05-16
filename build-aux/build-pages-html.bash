#!/usr/bin/env bash

# Rustdoc
ninja src/doc

# Help pages
ninja $(ninja -t targets rule CUSTOM_COMMAND | grep -E "^help.*\.page\$")

while read lang; do
    mkdir -p help-out/$lang
    yelp-build html help/$lang -o help-out/$lang
done < ../help/LINGUAS

mkdir help-out/C
yelp-build html ../help/C -o help-out/C