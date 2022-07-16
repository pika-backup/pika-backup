#!/usr/bin/env python3

import os
import subprocess

def main():
    print(f"{os.uname()=}")

    print(f"{subprocess.run(['ps', '-C', 'pika-backup-monitor'], capture_output=True)=}")
    
    flatpak_ps = subprocess.run(['ps', '-C', 'pika-backup-monitor'], capture_output=True).stdout
    
main()
