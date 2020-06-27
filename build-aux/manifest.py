#!/usr/bin/python3

import configparser
import sys

config = configparser.ConfigParser()
config.read('Cargo.toml')
print(config.get(sys.argv[1], sys.argv[2]).strip('"'), end='')
