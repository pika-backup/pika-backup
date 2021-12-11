# Changelog

## 0.4.0 Unreleased

- Fix not all data in mounted archives where always readable
- Fix backups cannot be aborted if backup source is unresponsive
- Change backup compression to the more effective "Zstandard" algorithm
- Change to accept ssh host keys for new hosts
- Change to no longer support migration from Pika Backup v0.2 configs
- Add message to user interface if target devive or server is unrepsonive

## 0.3.5 (2021-09-08)

- Fix metadata.

## 0.3.4 (2021-09-07)

- Fix metadata.
- Flatpak: Fix minor bugs in backup engine. (New borg-backup version.)

## 0.3.3 (2021-09-07)

- Update URLs in metadata.
- Add Dutch translation.
- Add Occitan translation.

## 0.3.2 (2021-05-28)

- Correct an error in the Spanish translation.
- Update Indonesian translation.
- Add Polish translation.

## 0.3.1 (2021-05-20)

- Fixes localization not working in flatpak
- Fixes existing env var LD_PRELOAD on host makes remote backups unusable

## 0.3.0 (2021-05-15)

- Adds basic explanation to URL field when creating new repository
- Adds button to unmount archives
- Adds estimate for remaining time to complete backup
- Adds translations to several new languages
- Adds warning for creating repositories on non-journaling file systems
- Changes backups to differentiate between warnings and errors
- Changes borg processes to lower priority
- Changes config file to be `backup.json` instead of `config.json`
- Changes history storage from config file to separate `history.json`
- Changes several messages to be more understandable
- Changes to a limited amount of borg reconnect attempts
- Changes to continue backups while ui is closed
- Changes to more concrete dialog responses than "No/Yes"

## 0.2.3 (2021-03-30)

- Adds flatpak options to support GNOME 40 hosts (GVfs 1.48)

## 0.2.2 (2021-03-23)

- Fixes archives listed with wrong creation time

## 0.2.1 (2021-01-05)

- Fixes crash on adding backup config with invalid URI
- Fixes list of archives for dark themes
- Adds translation to Swedish
- Adds keywords to .desktop-file

## 0.2.0 (2020-12-23)

- Adds translations to several languages
- Redesigns some parts of the user interface
- Shows backup status for each repository in overview
- Adds basis for supporting mobile clients
- Reduces idle CPU usage (stop invisible GtkSpinner's)
- Makes handling of encryption password requests faster and smother
- Adds caching for repository archives
- Adds basic support for GVfs enabling backups to sftp, smb etc.
- Adds custom command line options for borg to setup
- Enables easier onboarding for developers
- Many code cleanups

## 0.1.3 (2020-09-21)

- Shortens long error message and make info popover scrollable #42
- Updates to polished app icon by Jakub Steiner

## 0.1.2 (2020-09-09)

- Fixes data of other flatpak applications are missing in backups (~/.var/app) #39

## 0.1.1 (2020-08-28)

- Fixes missing removable device information after creating a new backup repository

## 0.1.0 (2020-08-25)

- First release
