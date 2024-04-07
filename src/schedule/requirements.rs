/*!
# Schedule execution criteria

Note: The term "last backup" includes failed backups.

## Intervals

### Hourly

Requirements

- Last backup is more than one hour ago. (Manual backups are considered here.)
- System is in use for more than [`crate::schedule::USED_THRESHOLD`]

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
use crate::prelude::*;
use crate::utils::upower::UPower;

/**
Global requirements

### Planned option

- Travel mode is not active
*/
#[derive(Debug, Clone, PartialEq)]
pub enum Global {
    /// Backup must not be running
    ThisBackupRunning,
    OtherBackupRunning(config::ConfigId),
    /// May not use metered connection
    MeteredConnection,
    OnBattery,
}

impl Global {
    /// Returns all requirements that are violated
    pub async fn check(config: &config::Backup, histories: &config::Histories) -> Vec<Self> {
        let mut vec = Vec::new();
        let settings = &config.schedule.settings;

        let running_backup = histories
            .iter()
            .filter(|(_, history)| history.is_running())
            .find(|(config_id, _)| {
                backup_config().try_get(config_id).map(|x| &x.repo_id) == Ok(&config.repo_id)
            });

        if let Some((running_config_id, _)) = running_backup {
            // TODO: Is this ever triggered?
            if *running_config_id == config.id {
                vec.push(Self::ThisBackupRunning)
            } else {
                vec.push(Self::OtherBackupRunning(running_config_id.clone()))
            }
        }

        if gio::NetworkMonitor::default().is_network_metered() && config.repo.is_internet().await {
            vec.push(Self::MeteredConnection)
        }

        if !settings.run_on_battery && UPower::on_battery().await == Some(true) {
            vec.push(Self::OnBattery)
        }

        vec
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Hint {
    DeviceMissing,
    NetworkMissing,
}

impl Hint {
    pub fn check(config: &config::Backup) -> Vec<Self> {
        let mut vec = Vec::new();

        if config.repo.is_drive_connected() == Some(false) {
            vec.push(Self::DeviceMissing)
        }

        if config.repo.is_network() && !gio::NetworkMonitor::default().is_network_available() {
            vec.push(Self::NetworkMissing)
        }

        vec
    }
}

#[derive(Debug, Clone)]
pub enum Due {
    NotDue { next: DateTime<Local> },
    Running,
}

#[derive(Debug, Clone, Serialize, Deserialize, zbus::zvariant::Type)]
pub enum DueCause {
    Regular,
    Retry,
}

impl Due {
    pub fn next_due(&self) -> Option<chrono::Duration> {
        match self {
            Self::NotDue { next } => Some(*next - chrono::Local::now()),
            Self::Running => None,
        }
    }

    /// Checks whether the schedule is due
    pub fn check(config: &config::Backup) -> Result<DueCause, Self> {
        Self::check_full(
            config,
            backup_history().try_get(&config.id).ok(),
            schedule_status().try_get(&config.id).ok(),
        )
    }

    fn check_full(
        config: &config::Backup,
        history: Option<&config::history::History>,
        activity: Option<&config::Activity>,
    ) -> Result<DueCause, Self> {
        Self::check_with_frequency(
            config.schedule.frequency,
            history,
            &activity.cloned().unwrap_or_default(),
            chrono::Local::now(),
        )
    }

    /// Same as check_full but does not rely on the current system time.
    ///
    /// Checks all the prerequisites in the history file and then delegates the actual
    /// implementation to [`check_real`] to allow for more comprehensive testing.
    fn check_with_frequency(
        schedule: config::Frequency,
        history: Option<&config::history::History>,
        activity: &config::Activity,
        now: chrono::DateTime<Local>,
    ) -> Result<DueCause, Self> {
        if history.is_some_and(|h| h.is_running()) {
            // Already running, skip
            return Err(Self::Running);
        };

        let Some(last_run) = history.and_then(|h| h.last_run()).map(|run| run.end) else {
            // Never ran before, always due
            return Ok(DueCause::Regular);
        };

        // The last successful backup
        let last_completed = history.and_then(|h| h.last_completed()).map(|run| run.end);

        Self::check_real(schedule, activity, now, last_run, last_completed)
    }

    /// The actual schedule calculation takes place here
    fn check_real(
        frequency: config::Frequency,
        activity: &config::Activity,
        now: chrono::DateTime<Local>,
        last_run: chrono::DateTime<Local>,
        last_completed: Option<chrono::DateTime<Local>>,
    ) -> Result<DueCause, Self> {
        // The next backup according to the regular schedule
        let next_run = Self::next_run(frequency, last_run);

        // Check if we are due for a regular backup
        if next_run <= now {
            // Check if the device has been in use for at least USED_THRESHOLD minutes. We ignore this for hourly.
            if matches!(frequency, config::Frequency::Hourly) || activity.is_threshold_reached() {
                return Ok(DueCause::Regular);
            } else {
                // We will be due soon, just wait a little longer until the device has been in use for a few more minutes
                return Err(Self::NotDue {
                    next: now + activity.time_until_threshold(),
                });
            }
        }

        // We are not technically due. We might however be eligible for a retry.
        //
        // Retries work on weekly and monthly backups. The idea is that if the last scheduled backup failed
        // we try again the day after to ensure a valid backup every week / month.
        match frequency {
            config::Frequency::Weekly { .. } | config::Frequency::Monthly { .. } => {
                // The next day after the last run, 00:00 (start of day)
                //
                // Panics: This is safe because we use a fixed offset and fixed offset don't have gaps
                let next_day_midnight = last_run
                    .fixed_offset()
                    .checked_add_days(chrono::Days::new(1))
                    .unwrap()
                    .with_time(chrono::NaiveTime::MIN)
                    .unwrap()
                    .with_timezone(&last_run.timezone());

                // We only allow one retry a day
                let next_retry = if let Some(completed) = last_completed {
                    // We use the time for the last completed backup to figure out the next retry
                    // the same way as we would with a regular scheduled backup.
                    Self::next_run(frequency, completed).max(next_day_midnight)
                } else {
                    // If we never completed a backup we try every day until we get one.
                    next_day_midnight
                };

                if next_retry <= now {
                    if activity.is_threshold_reached() {
                        Ok(DueCause::Retry)
                    } else {
                        Err(Self::NotDue {
                            next: now + activity.time_until_threshold(),
                        })
                    }
                } else {
                    // If the next retry is earlier than the next run we want the retry
                    Err(Self::NotDue { next: next_retry })
                }
            }
            _ => {
                // Retries are only allowed for weekly and monthly backups
                Err(Self::NotDue { next: next_run })
            }
        }
    }

    /// Determine the next scheduled backup time
    fn next_run(
        frequency: config::Frequency,
        last_run: chrono::DateTime<Local>,
    ) -> chrono::DateTime<Local> {
        let local_tz = last_run.timezone();

        match frequency {
            // Hourly backups just run every hour (measured after the last run end time)
            config::Frequency::Hourly => last_run + chrono::Duration::hours(1),
            // Daily backups run every day at the preferred time or later
            config::Frequency::Daily { preferred_time } => {
                // First we change the date if needed
                let next_date = if last_run.time() < preferred_time {
                    // Schedule for the same day because we ran before the preferred time
                    last_run
                } else {
                    // We already ran today on or after the scheduled time. Schedule for the next day.
                    last_run + chrono::Duration::days(1)
                };

                // Now we adjust the time to our preferred time.
                //
                // Panics: We use fixed_offset for the calculation because this lets
                // us ignore gaps in time (aka daylight savings time). According to
                // chrono docs this can only fail at the end of time and space.
                next_date
                    .fixed_offset()
                    .with_time(preferred_time)
                    .unwrap()
                    .with_timezone(&local_tz)
            }
            // Weekly backups run every week on or after the preferred day.
            // We schedule a backup for the preferred day regardless of whether
            // we already ran that week or not. This also allows us to completely
            // ignore the start of the week.
            config::Frequency::Weekly { preferred_weekday } => {
                let last_weekday = last_run.weekday();

                let last_weekday0 = last_weekday.num_days_from_monday();
                let pref_weekday0 = preferred_weekday.num_days_from_monday();

                // How many days do we need to add until we reach our preferred day?
                let offset_days = if pref_weekday0 <= last_weekday0 {
                    // Wait until next week
                    pref_weekday0 + 7 - last_weekday0
                } else {
                    // This week
                    pref_weekday0 - last_weekday0
                };

                // Add the offset to our last schedule
                let next_run = last_run + chrono::Duration::days(offset_days.into());

                // Panics: This only fails at the end of time because we use fixed_offset
                next_run
                    .fixed_offset()
                    .with_time(chrono::NaiveTime::MIN)
                    .unwrap()
                    .with_timezone(&local_tz)
            }
            // Monthly runs every month on or after the preferred day.
            // If the preferred day is not yet reached we schedule a backup for this month.
            config::Frequency::Monthly { preferred_day } => {
                let preferred_day = u32::from(preferred_day.min(31));

                // First we get the month right, with no regard for the correct day
                let next_month = if preferred_day <= last_run.day() {
                    // We need to run next month. checked_add_months will use the last day
                    // of the month if the day does not exist in the resulting month.
                    //
                    // Panics: This only fails at the end of time.
                    last_run.checked_add_months(chrono::Months::new(1)).unwrap()
                } else {
                    // We run this month
                    last_run
                };

                // Set the day. This will clamp to the last day of the month if the month is shorter.
                //
                // Panics: This only fails at the end of time
                let new_date = chronoutil::delta::with_day(next_month, preferred_day).unwrap();

                // Panics: This only fails at the end of time because we use fixed_offset
                new_date
                    .fixed_offset()
                    .with_time(chrono::NaiveTime::MIN)
                    .unwrap()
                    .with_timezone(&local_tz)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::schedule::USED_THRESHOLD;

    use super::*;

    #[test]
    fn test_check_running() {
        let config = config::Backup::test_new_mock();
        let mut history = config::history::History::default();
        let activity = config::Activity {
            used: USED_THRESHOLD,
            last_update: chrono::Local::now(),
        };

        history.start_running_now();

        let due = Due::check_full(&config, Some(&history), Some(&activity));
        matches::assert_matches!(due, Err(Due::Running));
    }

    #[test]
    fn test_check_daily() {
        let mut config = config::Backup::test_new_mock();
        let mut history = config::history::History::default();
        let activity = config::Activity {
            used: USED_THRESHOLD,
            last_update: chrono::Local::now(),
        };
        let preferred_time = chrono::Local::now().time() - chrono::Duration::hours(1);

        config.schedule.frequency = config::Frequency::Daily { preferred_time };

        // no activity

        history.insert(config::history::RunInfo::test_new_mock(
            chrono::Duration::hours(2),
        ));

        let due = Due::check_full(&config, Some(&history), None);
        assert!(match due {
            Err(Due::NotDue { next }) => {
                // due after device used enough
                assert!(
                    (chrono::Local::now() + chrono::Duration::from_std(USED_THRESHOLD).unwrap())
                        - next
                        < chrono::Duration::seconds(1)
                );
                true
            }
            _ => false,
        });

        // is due

        history.insert(config::history::RunInfo::test_new_mock(
            chrono::Duration::hours(2),
        ));

        let due = Due::check_full(&config, Some(&history), Some(&activity));
        matches::assert_matches!(due, Ok(DueCause::Regular));

        // failed today before preferred time

        history.insert(config::history::RunInfo::new_left_running(
            &(chrono::Local::now() - chrono::Duration::hours(2)),
        ));

        let due = Due::check_full(&config, Some(&history), Some(&activity));
        matches::assert_matches!(due, Ok(DueCause::Regular));

        // failed now, try again tomorrow

        let mut config_close = config;
        let preferred_time_close = chrono::Local::now().time() - chrono::Duration::seconds(1);

        let mut history_close = history.clone();
        history_close.insert(config::history::RunInfo::test_new_mock(
            chrono::Duration::seconds(1),
        ));

        config_close.schedule.frequency = config::Frequency::Daily {
            preferred_time: preferred_time_close,
        };

        history.insert(config::history::RunInfo::new_left_running(
            &chrono::Local::now(),
        ));

        let due = Due::check_full(&config_close, Some(&history_close), Some(&activity));
        assert!(match due {
            Err(Due::NotDue { next }) => {
                assert_eq!(
                    next,
                    chrono::Local::now()
                        .checked_add_days(chrono::Days::new(1))
                        .unwrap()
                        .with_time(preferred_time_close)
                        .unwrap()
                );
                true
            }
            _ => false,
        });

        // completed today

        history.clear();
        history.insert(config::history::RunInfo::test_new_mock(
            chrono::Duration::zero(),
        ));

        let due = Due::check_full(&config_close, Some(&history), Some(&activity));
        assert!(match due {
            Err(Due::NotDue { next }) => {
                assert_eq!(
                    next,
                    chrono::Local::now()
                        .checked_add_days(chrono::Days::new(1))
                        .unwrap()
                        .with_time(preferred_time_close)
                        .unwrap()
                );
                true
            }
            _ => false,
        });
    }

    #[test]
    fn test_check_weekly() {
        let mut config = config::Backup::test_new_mock();
        let mut history = Default::default();
        let activity = config::Activity {
            used: USED_THRESHOLD,
            last_update: chrono::Local::now(),
        };

        config.schedule.frequency = config::Frequency::Weekly {
            preferred_weekday: (chrono::Local::now() - chrono::Duration::days(1)).weekday(),
        };

        // Never ran

        let due = Due::check_full(&config, Some(&history), Some(&activity));
        matches::assert_matches!(due, Ok(DueCause::Regular));

        // no activity

        history.insert(config::history::RunInfo::new_left_running(
            &(chrono::Local::now() - chrono::Duration::days(1)),
        ));

        let due = Due::check_full(&config, Some(&history), None);
        assert!(match due {
            Err(Due::NotDue { next }) => {
                // due after device used enough
                assert!(
                    (chrono::Local::now() + chrono::Duration::from_std(USED_THRESHOLD).unwrap())
                        - next
                        < chrono::Duration::seconds(1)
                );
                true
            }
            _ => false,
        });

        // due yesterday and failed yesterday

        let due = Due::check_full(&config, Some(&history), Some(&activity));
        matches::assert_matches!(due, Ok(DueCause::Retry));

        // due yesterday and completed yesterday

        history.insert(config::history::RunInfo::test_new_mock(
            chrono::Duration::days(1),
        ));

        let due = Due::check_full(&config, Some(&history), Some(&activity));

        assert!(match due {
            Err(Due::NotDue { next }) => {
                assert_eq!(
                    next,
                    (chrono::Local::now() + chrono::Duration::days(6))
                        .with_time(chrono::NaiveTime::MIN)
                        .unwrap()
                );
                true
            }
            _ => false,
        });

        // due today and only completed yesterday

        config.schedule.frequency = config::Frequency::Weekly {
            preferred_weekday: chrono::Local::now().weekday(),
        };

        let due = Due::check_full(&config, Some(&history), Some(&activity));
        matches::assert_matches!(due, Ok(DueCause::Regular));

        // due today and completed today

        history.insert(config::history::RunInfo::test_new_mock(
            chrono::Duration::zero(),
        ));

        let due = Due::check_full(&config, Some(&history), Some(&activity));

        assert!(match due {
            Err(Due::NotDue { next }) => {
                assert_eq!(
                    next,
                    chrono::Local::now()
                        .with_time(chrono::NaiveTime::MIN)
                        .unwrap()
                        + chrono::Duration::weeks(1)
                );
                true
            }
            _ => false,
        });
    }

    #[test]
    fn test_check_monthly() {
        let mut config = config::Backup::test_new_mock();
        let mut history = config::history::History::default();
        let activity = config::Activity {
            used: USED_THRESHOLD,
            last_update: chrono::Local::now(),
        };

        let preferred_day = chrono::Local::now() - chrono::Duration::days(1);
        config.schedule.frequency = config::Frequency::Monthly {
            preferred_day: preferred_day.day() as u8,
        };

        // due yesterday and failed now

        history.insert(config::history::RunInfo::new_left_running(
            &(preferred_day.with_time(chrono::NaiveTime::MIN).unwrap()
                + chrono::Duration::seconds(1)),
        ));

        let due = Due::check_full(&config, Some(&history), Some(&activity));
        matches::assert_matches!(due, Ok(DueCause::Retry));

        // Completed yesterday
        history.insert(config::history::RunInfo::test_new_mock(
            chrono::Duration::days(1),
        ));

        let due = Due::check_full(&config, Some(&history), Some(&activity));

        assert!(match due {
            Err(Due::NotDue { next }) => {
                assert_eq!(
                    next,
                    chronoutil::delta::shift_months(preferred_day, 1)
                        .with_time(chrono::NaiveTime::MIN)
                        .unwrap()
                );
                true
            }
            _ => false,
        });
    }
}
