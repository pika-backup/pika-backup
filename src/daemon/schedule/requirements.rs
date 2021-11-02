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
            if *running_config_id != config.id {
                vec.push(Self::OtherBackupRunning(running_config_id.clone()))
            }
        }

        if config.repo.is_network() && gio::NetworkMonitor::default().is_network_metered() {
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
    NotDueDateTime { next: DateTime<Local> },
    NotDueDate { next: Date<Local> },
    Running,
}

impl Due {
    pub fn next_due(&self) -> Option<chrono::Duration> {
        match self {
            Self::NotDueDateTime { next } => Some(*next - chrono::Local::now()),
            Self::NotDueDate { next } => Some(*next - chrono::Local::today()),
            Self::Running => None,
        }
    }

    pub fn check(config: &config::Backup) -> Result<(), Self> {
        let schedule = &config.schedule;

        let history_all = BACKUP_HISTORY.load();
        let history = history_all.get_result(&config.id).ok();

        if history.map(|x| x.running.is_some()) == Some(true) {
            Err(Self::Running)
        } else if let Some(last_run) = history.and_then(|x| x.run.front()) {
            let last_completed = history.and_then(|x| x.last_completed.as_ref());

            let last_run_datetime = last_run.end;

            debug!("Last run: {:?}", last_run_datetime);
            debug!("Last completed: {:?}", last_completed.map(|x| x.end));

            let last_run_ago = chrono::Local::now() - last_run.end;

            let activity = SCHEDULE_STATUS
                .load()
                .activity
                .get(&config.id)
                .map(|x| x.used)
                .unwrap_or_default();

            match schedule.frequency {
                config::Frequency::Hourly => {
                    if last_run_ago >= chrono::Duration::hours(1) {
                        Ok(())
                    } else {
                        debug!(
                            "Last backup is only {} minutes ago.",
                            last_run_ago.num_minutes()
                        );
                        Err(Self::NotDueDateTime {
                            next: last_run.end + chrono::Duration::hours(1),
                        })
                    }
                }
                config::Frequency::Daily { preferred_time } => {
                    let scheduled_datetime =
                        chrono::Local::today().and_time(preferred_time).unwrap();
                    let scheduled_datetime_before = scheduled_datetime - chrono::Duration::days(1);

                    if last_run_datetime < scheduled_datetime
                        && scheduled_datetime < chrono::Local::now()
                    {
                        // regular backup
                        Ok(())
                    } else if scheduled_datetime_before > last_run_datetime
                        && activity >= super::USED_THRESHOLD
                    {
                        // catch-up backup
                        Ok(())
                    } else {
                        let next = if chrono::Local::now() < scheduled_datetime {
                            scheduled_datetime
                        } else {
                            scheduled_datetime + chrono::Duration::days(1)
                        };

                        Err(Self::NotDueDateTime { next })
                    }
                }
                config::Frequency::Weekly { preferred_weekday } => {
                    let day_difference = chrono::Local::today().weekday().number_from_sunday()
                        as i64
                        - preferred_weekday.number_from_sunday() as i64;
                    let scheduled_date =
                        chrono::Local::today() - chrono::Duration::days(-day_difference.abs());

                    if chrono::Local::today() >= scheduled_date
                        && last_run_datetime.date() < scheduled_date
                        && activity >= super::USED_THRESHOLD
                    {
                        Ok(())
                    } else {
                        let next = if chrono::Local::today() < scheduled_date {
                            scheduled_date
                        } else {
                            scheduled_date + chrono::Duration::weeks(1)
                        };

                        Err(Self::NotDueDate { next })
                    }
                }
                config::Frequency::Monthly { preferred_day } => {
                    let scheduled_date = chrono::Local::today()
                        .with_day(preferred_day as u32)
                        .unwrap_or_else(chrono::Local::today);

                    let scheduled_date_before = chronoutil::delta::shift_months(scheduled_date, -1);

                    if chrono::Local::today() >= scheduled_date
                        && last_run_datetime.date() < scheduled_date
                    {
                        Ok(())
                    } else if chrono::Local::today() >= scheduled_date_before
                        && last_run_datetime.date() < scheduled_date_before
                    {
                        Ok(())
                    } else {
                        let next = if chrono::Local::today() < scheduled_date {
                            scheduled_date
                        } else {
                            chronoutil::delta::shift_months(scheduled_date, 1)
                        };

                        Err(Self::NotDueDate { next })
                    }
                }
            }
        } else {
            // never ran before
            Ok(())
        }
    }
}
