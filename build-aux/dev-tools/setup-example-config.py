#!/usr/bin/env python3

import datetime
import os
import random

config_dir = os.path.expanduser("~/.var/app/org.gnome.World.PikaBackup.Devel/config/pika-backup")

### Weekly HDD backup ###

hdd_last_backup = (datetime.datetime.now(datetime.timezone.utc) - datetime.timedelta(2)).isoformat()

hdd_config = """
  {
    "config_version": 2,
    "id": "882a1713-c273-4f2f-9d5b-a106559cf245",
    "archive_prefix": "73e3f2-",
    "repo_id": "a2538db8086221062fa5467d90169efcf2a84ee57e221adb3b03ab32f0e4ac2b",
    "repo": {
      "type": "Local",
      "path": "backup-desktop",
      "mount_path": "/run/media/herold/Backup WD",
      "uri": "file:///run/media/herold/Backup%20WD/backup-desktop",
      "drive_name": "WDC WD7500BMVW-11AJGS2",
      "mount_name": "Backup WD",
      "volume_uuid": "71593ea0-53b7-4440-9528-eb393f4a1a69",
      "volume_uuid_identifier": "51a28a36-c0bb-452b-81ba-e300fe736aff",
      "removable": true,
      "icon": ". GEmblemedIcon .%20GThemedIcon%20drive-harddisk-usb%20drive-harddisk%20drive%20drive-harddisk-usb-symbolic%20drive-harddisk-symbolic%20drive-symbolic .%20GEmblem%20changes-allow%201",
      "icon_symbolic": ". GThemedIcon drive-harddisk-usb-symbolic drive-harddisk-symbolic drive-symbolic drive-harddisk-usb drive-harddisk drive",
      "settings": null
    },
    "encrypted": false,
    "encryption_mode": "repokey",
    "include": [
      ""
    ],
    "exclude": [
      "Caches",
      "FlatpakApps",
      "VmsContainers"
    ],
    "schedule": {
      "enabled": true,
      "settings": {
        "run_on_battery": false
      },
      "frequency": {
        "Weekly": {
          "preferred_weekday": "Wed"
        }
      }
    },
    "prune": {
      "enabled": true,
      "keep": {
        "hourly": 48,
        "daily": 14,
        "weekly": 4,
        "monthly": 12,
        "yearly": 10
      }
    },
    "title": "",
    "user_scripts": {}
  }
"""

hdd_history = """
 "882a1713-c273-4f2f-9d5b-a106559cf245": {
    "config_version": 2,
    "run": [
      {
        "end": "%s",
        "outcome": {
          "Completed": {
            "stats": {
              "archive": {
                "duration": 0.50228,
                "id": "24250b62669bb824e93ffe96895e7a60e45ef4435d4b063ab21542ea76d28a6e",
                "name": "73e3f2-59465ea3",
                "stats": {
                  "compressed_size": 119409786,
                  "deduplicated_size": 119409786,
                  "nfiles": 30,
                  "original_size": 119831854
                }
              }
            }
          }
        },
        "messages": [],
        "include": [],
        "exclude": []
      }
    ],
    "running": null,
    "browsing": null,
    "last_completed": {
      "end": "%s",
      "outcome": {
        "Completed": {
          "stats": {
            "archive": {
              "duration": 0.50228,
              "id": "24250b62669bb824e93ffe96895e7a60e45ef4435d4b063ab21542ea76d28a6e",
              "name": "73e3f2-59465ea3",
              "stats": {
                "compressed_size": 119409786,
                "deduplicated_size": 119409786,
                "nfiles": 30,
                "original_size": 119831854
              }
            }
          }
        }
      },
      "messages": [],
      "include": [],
      "exclude": []
    },
    "last_check": null,
    "suggested_exclude": {
      "PermissionDenied": []
    }
  }
""" % (hdd_last_backup, hdd_last_backup)

### Hourly Remote Backup ###

remote_last_backup = (
    datetime.datetime.now(datetime.timezone.utc) - datetime.timedelta(minutes=47)
).isoformat()


remote_config = """
  {
    "config_version": 2,
    "id": "a02ffb37-feb1-412b-af09-4f850125b313",
    "archive_prefix": "5e7cf8-",
    "repo_id": "00f7f2b59074c4f900d56607c6923747828e79d5407660a22f7b8eefeb3a149f",
    "repo": {
      "type": "Remote",
      "uri": "ssh://backup.example.org/./repo",
      "settings": null
    },
    "encrypted": true,
    "encryption_mode": "repokey",
    "include": [
      "Documents",
      "Music",
      "Pictures"
    ],
    "exclude": [
      "Caches"
    ],
    "schedule": {
      "enabled": true,
      "settings": {
        "run_on_battery": false
      },
      "frequency": "Hourly"
    },
    "prune": {
      "enabled": false,
      "keep": {
        "hourly": 48,
        "daily": 14,
        "weekly": 4,
        "monthly": 12,
        "yearly": 10
      }
    },
    "title": "",
    "user_scripts": {}
  }
"""

remote_history = """
"a02ffb37-feb1-412b-af09-4f850125b313": {
    "config_version": 2,
    "run": [
      {
        "end": "%s",
        "outcome": {
          "Completed": {
            "stats": {
              "archive": {
                "duration": 202.450195,
                "id": "30b7020ba2b915a39d04703ef9c6bc6d2dd14301e884659e9087d34edfcc01ba",
                "name": "5e7cf8-ffb08911",
                "stats": {
                  "compressed_size": 5806639,
                  "deduplicated_size": 777,
                  "nfiles": 1,
                  "original_size": 5857125
                }
              }
            }
          }
        },
        "messages": [],
        "include": [],
        "exclude": []
      }
    ],
    "running": null,
    "browsing": null,
    "last_completed": {
      "end": "%s",
      "outcome": {
        "Completed": {
          "stats": {
            "archive": {
              "duration": 202.450195,
              "id": "30b7020ba2b915a39d04703ef9c6bc6d2dd14301e884659e9087d34edfcc01ba",
              "name": "5e7cf8-ffb08911",
              "stats": {
                "compressed_size": 5806639,
                "deduplicated_size": 777,
                "nfiles": 1,
                "original_size": 5857125
              }
            }
          }
        }
      },
      "messages": [],
      "include": [],
      "exclude": []
    },
    "last_check": null,
    "suggested_exclude": {
      "PermissionDenied": []
    }
  }
""" % (remote_last_backup, remote_last_backup)

config = f"[ {remote_config}\n,{hdd_config} ]\n"

history = f"{{ {remote_history}\n,{hdd_history} }}\n"

config_config_dir = config_dir + "/backup.json"
history_config_dir = config_dir + "/history.json"

rand = str(random.randrange(10000000))

try:
    os.rename(config_config_dir, config_config_dir + ".bak." + rand)
except Exception as e:
    print(e)
    pass

try:
    os.rename(history_config_dir, history_config_dir + ".bak." + rand)
except Exception as e:
    print(e)
    pass

with open(config_config_dir, "w") as f:
    f.write(config)

with open(history_config_dir, "w") as f:
    f.write(history)
