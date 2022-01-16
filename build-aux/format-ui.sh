#!/bin/sh

gtk4-builder-tool simplify --replace src/ui/*.ui
rpl 'translatable="1"' 'translatable="yes"' src/ui/*.ui
