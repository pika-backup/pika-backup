/*!
# Schedule execution criteria

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

use chrono::prelude::*;
use gio::prelude::*;

use crate::config;
use crate::daemon::prelude::*;

/**
Global requirements

### Planned option

- Travel mode is not active
- On battery (optional?)
*/
#[derive(Debug, Clone)]
pub enum Global {
    /// Backup must not be running
    ThisBackupRunning,
    OtherBackupRunning(config::ConfigId),
    /// May not use metered connection
    MeteredConnection,
}

impl Global {
    /// If any it returns the first requirement that is violated
    pub fn check(config: &config::Backup) -> Vec<Self> {
        let mut vec = Vec::new();

        let history = BACKUP_HISTORY.load();

        let running_backup = history
            .as_ref()
            .iter()
            .filter(|(_, history)| history.running.is_some())
            .find(|(config_id, _)| {
                BACKUP_CONFIG
                    .load()
                    .get_result(config_id)
                    .map(|x| &x.repo_id)
                    == Ok(&config.repo_id)
            });

        if let Some((running_config_id, _)) = running_backup {
            if *running_config_id == config.id {
                vec.push(Self::ThisBackupRunning)
            } else {
                vec.push(Self::OtherBackupRunning(running_config_id.clone()))
            }
        }

        if config.repo.is_network() && gio::NetworkMonitor::default().is_network_metered() {
            // TODO: Does not seem to work inside Flatpak
            vec.push(Self::MeteredConnection)
        }

        vec
    }
}

#[derive(Debug, Clone)]
pub enum Hint {
    DeviceMissing,
}

impl Hint {
    pub fn check(config: &config::Backup) -> Vec<Self> {
        let mut vec = Vec::new();

        if config.repo.is_drive_connected() == Ok(false) {
            vec.push(Self::DeviceMissing)
        }

        vec
    }
}

#[derive(Debug, Clone)]
pub enum Due {
    NotDue { next: DateTime<Local> },
}

impl std::fmt::Display for Due {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::NotDue { next } => write!(
                f,
                "{}",
                // TODO: gettext string
                format!(
                    "Next Scheduled Backup: {}",
                    glib::DateTime::from_unix_local(next.timestamp())
                        .unwrap()
                        .format("%c")
                        .unwrap()
                        .to_string()
                )
            ),
        }
    }
}

impl Due {
    pub fn check(config: &config::Backup) -> Result<(), Self> {
        let schedule = &config.schedule;

        let history_all = BACKUP_HISTORY.load();
        let history = history_all.get_result(&config.id).ok();

        if let Some(last_run) = history.and_then(|x| x.run.front()) {
            let last_completed = history.and_then(|x| x.last_completed.as_ref());

            let last_run_datetime = last_run.end;

            debug!("Last run: {:?}", last_run_datetime);
            debug!("Last completed: {:?}", last_completed.map(|x| x.end));

            let last_run_ago = chrono::Local::now() - last_run.end;

            match schedule.frequency {
                config::Frequency::Hourly => {
                    if last_run_ago >= chrono::Duration::hours(1) {
                        Ok(())
                    } else {
                        debug!(
                            "Last backup is only {} minutes ago.",
                            last_run_ago.num_minutes()
                        );
                        Err(Self::NotDue {
                            next: last_run.end + chrono::Duration::hours(1),
                        })
                    }
                }
                config::Frequency::Daily { preferred_time } => {
                    let scheduled_datetime =
                        chrono::Local::today().and_time(preferred_time).unwrap();
                    if last_run_datetime < scheduled_datetime
                        && scheduled_datetime < chrono::Local::now()
                    {
                        Ok(())
                    } else {
                        Err(Self::NotDue {
                            next: chrono::Local::now(),
                        })
                    }
                }
                // TODO
                _ => {
                    error!("Not supported yet.");
                    Ok(())
                }
            }
        } else {
            // never ran before
            Ok(())
        }
    }
}
