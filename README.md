# Pika Backup

Doing backups the easy way. Plugin your USB drive and let the Pika do the rest for you.

[<img width='240' alt='Download on Flathub' src='https://flathub.org/assets/badges/flathub-badge-en.png' />](https://flathub.org/apps/details/org.gnome.World.PikaBackup)

[<img width='240' alt='Vote for Features' src='https://img.shields.io/badge/Participate%20On-Feature%20Upvote-323a46?style=flat' />](https://pika-backup.featureupvote.com/)

[<img width='240' alt='Part of GNOME Circle' src='https://gitlab.gnome.org/Teams/Circle/-/raw/91de93edbb3e75eb0882d56bd466e58b525135d5/assets/button/circle-button-fullcolor.svg' />](https://flathub.org/apps/details/org.gnome.World.PikaBackup)

### Features

<ul>
      <li>Setup new backup repositories or uses existing ones</li>
      <li>Create backups locally and remote</li>
      <li>Save time and disk space because Pika Backup does not need to copy known data again</li>
      <li>Encrypt your backups</li>
      <li>List created archives and browse through their contents</li>
      <li>Recover files or folders via your file browser</li>
    </ul>

Pika Backup is powered by the well-tested borg-backup software.

### Limitations
  
Currently, scheduled backups are not supported. Excluding files from a backup via regular expressions and alike is not implemented yet. Remote backup locations must support SSH and need to have a borg-backup binary installed.

![Pika Backup Setup](/uploads/596347a2e99be37c3f8a035b75cea8ea/pika-pile-1.png)

### Alternative software

- [Vorta](https://flathub.org/apps/details/com.borgbase.Vorta), borg-backup as backend, supports scheduled backups, Qt frontend for advanced users
- [Déjà Dup Backups](https://flathub.org/apps/details/org.gnome.DejaDup), duplicity (librsync) as backend, supports scheduled backups, GTK frontend
