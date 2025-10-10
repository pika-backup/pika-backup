# Changelog

## 0.8.0 (unreleased)

- Fix timestamps not respecting 12h/24h clock format setting
- Fix UI sometimes being unresponsive when starting / stopping backups
- Add detailed explanation about the risks and benefits of encrypting backups
- Change fnmatches to be stored as strings instead of bytes in config
- Change setup dialog start page is split into two pages
- Change setup dialog design to look more modern
- Change history file to be smaller
- Change to use AdwAlertDialog
- Change to use AdwAboutDialog
- Change to use AdwDialog for all dialogs
- Add propper error messages for more situations like missing filesystem access
- Add ability to backup files

## 0.7.5 (2025-10-10)

- Fix missing icons for GNOME 49 runtime

## 0.7.4 (2024-09-23)
- Fix borg version comparison

## 0.7.3 (2024-07-01)

- Fix backups being aborted on battery if running on battery was explicitly allowed
- Fix local backups are considered metered
- Fix user script variable $FROM_SCHEDULE not being set correctly

## 0.7.2 (2024-04-10)

- Fix shell scripts not inheriting environment variables which broke eg. `notify-send`
- Fix crash when clicking on deactivated buttons on schedule page

## 0.7.1 (2024-03-11)

- Fix schedule times / days resetting to random values when the schedule page is opened
- Fix set CPU scheduling priority of backup process

## 0.7.0 (2024-03-02)

- Fix filesystem was unmounted via other means error when trying to unmount
- Fix don't consider already mounted an error
- Fix non-create operations would not postpone the schedule
- Fix newly inserted volumes would sometimes not be detected to contain backup repositories
- Fix free space lookup doesn't use the SSH port from the backup configuration
- Fix missing device dialog would appear on top of other dialogs
- Fix no time/day preselected when changing schedule frequency mode
- Change setup dialog to use new design
- Change SMB mount error to be more descriptive
- Change history log to include archive mounting errors
- Change behavior on missing keyring daemon: Save passwords in memory until close
- Change to offer to stop browsing archive for every borg task
- Change file and folder choosers to preselect a folder
- Change backup notifications to be dismissed when a new backup starts
- Change local borg repository config to set additional_free_space to 2G
- Add docker to containers and VMs exclusion preset
- Add per-backup preferences window
- Add ability to set custom titles for backup configs
- Add ability to edit regex and shell exclusion patterns
- Add ability to run shell scripts on certain backup events
- Add ability to change the backup encryption password
- Add preference to allow running scheduled backups on battery
- Add exclusion suggestions for unreadable paths
- Add eject button for backup disk
- Add function to check archives integrity
- Add restart functionality to monitor process after app updates

## 0.6.2 (2023-09-06)

- Fix an error where reconnection would silently fail

## 0.6.1 (2023-04-21)

- Fix archives can not be mounted on some systems
- Fix removable drives can not be added to a backup

## 0.6.0 (2023-04-13)

- Fix compact is not run after prune
- Fix potential crash after deleting archives
- Fix spurious 'Pika Backup crashed' messages
- Fix size / free space estimation for some remote repositories
- Fix issues with scheduled monthly backups scheduled for the end of the month
- Fix secret service errors aborting config creation / deletion
- Fix backup doesn't abort until reconnection timeout has been reached
- Fix some dialogs are not closable with escape
- Fix overview does sometimes not show the backup config when only one config is present
- Fix app does not quit when a non-backup operation is finished with the window closed
- Change secret service error messages to include specific instructions how to resolve the issue
- Change running in background error messages to explain what it means when the background portal is not available
- Change to explain checkpoint creation when aborting backups
- Change to restart backup after SSH connection timeout
- Change reconnection to be abortable and count down seconds remaining
- Change archive mount permissions to more accurately reflect the permissions saved in the archive
- Change abort messages to more accurately reflect the current operation
- Change to explain the difference between 'Amount saved' and 'Backup space used'
- Add background portal status API messages about running operations
- Add ability to answer questions from borg process
- Add explanatory message when adding sandboxed paths to a backup configuration

## 0.5.2 (2023-02-23)

- Fix spelling mistake in daemon app id constant

## 0.5.1 (2023-02-21)

- Fix a panic during backup size estimates for files that have invalid time metadata

## 0.5.0 (2023-02-19)

- Change desktop entry category from "System" to "Utility/Archiving"
- Change prune section to only ask for confirmation if more archives could be deleted
- Change some of the setup dialogs design
- Change status icon colors for better contrast
- Change to always use Adwaita icon theme
- Change to hide advanced settings behind toggle button in setup
- Change to only list mounted SMB and SFTP locations in setup dialog
- Change to only mention SMB and SFTP schemas in setup dialog
- Change to use AdwAboutWindow
- Change to use AdwMessageDialog
- Change to use XDG runtime dir on host instead of HOME for mounts
- Change to symbolic icons in include/exclude list
- Add ability to add multiple includes/excludes at once from file chooser
- Add ability to delete single archives
- Add automatic mounting of unmounted drives
- Add debug information to about window
- Add detection of renamed drives
- Add guessed history entry when transferring config
- Add missing standard shortcuts
- Add predefined exclusion rules
- Add regular expressions and shell pattern exclude support
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
