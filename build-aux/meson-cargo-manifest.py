#!/usr/bin/python3

import tomllib
import sys

with open('Cargo.toml', 'rb') as f:
    toml = tomllib.load(f)
    print(toml[sys.argv[1]][sys.argv[2]], end='')
