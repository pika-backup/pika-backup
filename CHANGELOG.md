# Changelog

## 0.5.0 Unreleased

- Change desktop entry category from "System" to "Utility/Archiving"
- Change prune section to only ask for confirmation if more archives could be deleted
- Change some of the setup dialogs design
- Change status icon colors for better contrast
- Change to always use Adwaita icon theme
- Change to hide advanced settings behind toggle button in setup
- Change to only list mounted SMB and SFTP locations in setup dialog
- Change to only mention SMB and SFTP schemas in setup dialog
- Change to use AdwAboutWindow
- Change to use AdwEntryRow
- Change to use AdwMessageDialog
- Change to use XDG runtime dir on host instead of HOME for mounts
- Change to symbolic icons in include/exclude list
- Add ability to add multiple includes/excludes at once from file chooser
- Add automatic mounting of unmounted drives
- Add debug information to about window
- Add detection of renamed drives
- Add guessed history entry when transferring config
- Add missing standard shortcuts
- Add predefined exclusion rules
- Add regular expressions and shell pattern exclude support
- Add support for CACHEDIR.TAG exclusion
- Remove special representation with switch for Home folder

## 0.4.2 (2022-07-12)

- Fix again some xdg-desktop-portal versions writing broken autostart files (workaround)
- Add mobile device support indication in metadata

## 0.4.1 (2022-06-24)

- Fix some xdg-desktop-portal versions writing broken autostart files (workaround)
- Change to borg version 1.2.1 (flatpak)
- Change status labels to use fixed width numbers
- Add support for new 'finished' archive progress field in borg 1.2.1

## 0.4.0 (2022-05-15)

- Fix backups cannot be aborted if backup source is unresponsive
- Fix memory leak from setup dialog
- Fix not all data in mounted archives where always readable
- Fix sometimes free space of the wrong device is reported for remote repositories
- Change backup compression to the more effective "Zstandard" algorithm
- Change design of many UI components
- Change setup dialog to include password question
- Change to GTK 4 and libadwaita for frontend
- Change to a different password storage
- Change to a new app icon by @bertob
- Change to accept ssh host keys for new hosts
- Change to mount the flatpak dir (xdg-data/flatpak) inside of flatpaks
- Change to require at least borg version 1.2
- Change to use faster blake2 hash algorithm if no SHA256 CPU instruction is available
- Add archive name prefixes
- Add better error messages for incorrect remote locations
- Add help with information about recovery
- Add message to inform user about what task is locking the repo
- Add message to user interface if target devive or server is unrepsonive
- Add setup option to inherit include, exclude, and prefix from existing archives
- Add several mnemonics
- Add shortcuts dialog
- Add support for manually and automatically deleting old archives
- Add support for scheduled backups
- Add support for user@host:path borg remote location syntax
- Remove option to not store encryption passwords in secrete service
- Remove support for migrating from Pika Backup v0.2 configs

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
