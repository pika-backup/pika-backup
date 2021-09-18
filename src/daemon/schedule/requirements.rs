/*!
# Scheduled backups

Note: The term "last backup" includes failed backups.

## Intervals

### Hourly

Requirements

- Last backup is more than one hour ago. (Manual backups are considered here.)
- System is in use for more than [`MIN_USAGE`]

### Daily

Daily backups try to ensure that a backup exists for every day the system is used.

- A regular backup is started past [preferred_time] if no scheduled backup for this day exists.
- A catch-up backup is started if no backup exists after the last active days [preferred_time].
- A completion backup is started if a scheduled backup exists for the current day but it was before the [preferred_time].
    - We do the backup again if the [preferred_time] is changed to a later time in the config.
    - We supplement completion backups with the designated backup of the current day.

[preferred_time]: crate::config::Frequency::Daily::preferred_time

### Weekly

- Retried every day after failure.

### Monthly

- Retried every day after failure.

*/

use gio::prelude::*;

use super::init::BACKUP_STATUS;
use crate::config;

/**
Global requirements

### Planned option

- Travel mode is not active
- On battery (optional?)
*/
#[derive(Debug)]
pub enum Global {
    /// Backup must not be running
    ThisBackupRunning,
    OtherBackupRunning(config::ConfigId),
    /// May not use metered connection
    MeteredConnection,
}

impl Global {
    /// If any it returns the first requirement that is violated
    pub fn check(config: &config::Backup) -> Result<(), Self> {
        let status = BACKUP_STATUS.load();
        let running_backup = status
            .as_ref()
            .as_ref()
            .and_then(|x| x.get(&config.repo_id));

        // TODO determine of this or other backup running
        if let Some(config_id) = running_backup {
            if *config_id == config.id {
                Err(Self::ThisBackupRunning)
            } else {
                Err(Self::OtherBackupRunning(config_id.clone()))
            }
        } else if config.repo.is_network() && gio::NetworkMonitor::default().is_network_metered() {
            Err(Self::MeteredConnection)
        } else {
            Ok(())
        }
    }
}
