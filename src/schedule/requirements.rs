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
#[derive(Debug, Clone)]
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

        let running_backup = histories
            .iter()
            .filter(|(_, history)| history.running.is_some())
            .find(|(config_id, _)| {
                backup_config().get_result(config_id).map(|x| &x.repo_id) == Ok(&config.repo_id)
            });

        if let Some((running_config_id, _)) = running_backup {
            // TODO: Is this ever triggered?
            if *running_config_id == config.id {
                vec.push(Self::ThisBackupRunning)
            } else {
                vec.push(Self::OtherBackupRunning(running_config_id.clone()))
            }
        }

        if gio::NetworkMonitor::default().is_network_metered()
            && config.repo.is_host_local().await == Some(false)
        {
            vec.push(Self::MeteredConnection)
        }

        if UPower::on_battery().await == Some(true) {
            vec.push(Self::OnBattery)
        }

        vec
    }
}

#[derive(Debug, Clone)]
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

    pub fn check(config: &config::Backup) -> Result<DueCause, Self> {
        Self::check_full(
            config,
            backup_history().get_result(&config.id).ok(),
            schedule_status().get_result(&config.id).ok(),
        )
    }

    pub fn check_full(
        config: &config::Backup,
        history: Option<&config::history::History>,
        activity: Option<&config::Activity>,
    ) -> Result<DueCause, Self> {
        let schedule = &config.schedule;
        let activity = activity.map(|x| x.used).unwrap_or_default();
        let last_completed = history.and_then(|x| x.last_completed.as_ref());

        if history.map(|x| x.running.is_some()) == Some(true) {
            Err(Self::Running)
        } else if let Some(last_run) = history.and_then(|x| x.run.front()) {
            match schedule.frequency {
                config::Frequency::Hourly => {
                    let last_run_ago = chrono::Local::now() - last_run.end;
                    if last_run_ago >= chrono::Duration::hours(1) {
                        Ok(DueCause::Regular)
                    } else {
                        Err(Self::NotDue {
                            next: last_run.end + chrono::Duration::hours(1),
                        })
                    }
                }
                config::Frequency::Daily { preferred_time } => {
                    let now = chrono::Local::now();

                    let scheduled_datetime = {
                        let datetime = now
                            .date()
                            .and_time(preferred_time)
                            .unwrap_or_else(|| now.date().pred().and_hms(0, 0, 0));

                        if datetime > now {
                            datetime - chrono::Duration::days(1)
                        } else {
                            datetime
                        }
                    };

                    if last_run.end < scheduled_datetime {
                        if activity >= super::USED_THRESHOLD {
                            Ok(DueCause::Regular)
                        } else {
                            Err(Self::NotDue {
                                next: chrono::Local::now()
                                    + chrono::Duration::from_std(super::USED_THRESHOLD - activity)
                                        .unwrap_or_else(|_| chrono::Duration::zero()),
                            })
                        }
                    } else {
                        Err(Self::NotDue {
                            next: scheduled_datetime + chrono::Duration::days(1),
                        })
                    }
                }
                config::Frequency::Weekly { preferred_weekday } => {
                    let today = chrono::Local::today();

                    let scheduled_date = {
                        let iso_week = today.iso_week();
                        let schedule_date = chrono::Local.isoywd(
                            iso_week.year(),
                            iso_week.week(),
                            preferred_weekday,
                        );

                        if schedule_date > today {
                            schedule_date - chrono::Duration::weeks(1)
                        } else {
                            schedule_date
                        }
                    };

                    if last_run.end.date() < scheduled_date {
                        if activity >= super::USED_THRESHOLD {
                            Ok(DueCause::Regular)
                        } else {
                            Err(Self::NotDue {
                                next: chrono::Local::now()
                                    + chrono::Duration::from_std(super::USED_THRESHOLD - activity)
                                        .unwrap_or_else(|_| chrono::Duration::zero()),
                            })
                        }
                    } else if last_completed.map(|x| x.end.date()) < Some(scheduled_date) {
                        if last_run.end.date() == today {
                            let next = last_run.end.date().succ().and_hms(0, 0, 0);
                            Err(Self::NotDue { next })
                        } else if activity < super::USED_THRESHOLD {
                            Err(Self::NotDue {
                                next: chrono::Local::now()
                                    + chrono::Duration::from_std(super::USED_THRESHOLD - activity)
                                        .unwrap_or_else(|_| chrono::Duration::zero()),
                            })
                        } else {
                            Ok(DueCause::Retry)
                        }
                    } else {
                        Err(Self::NotDue {
                            next: scheduled_date.and_hms(0, 0, 0) + chrono::Duration::weeks(1),
                        })
                    }
                }
                config::Frequency::Monthly { preferred_day } => {
                    let scheduled_date = chrono::Local::today()
                        .with_day(preferred_day as u32)
                        .unwrap_or_else(chrono::Local::today);

                    let scheduled_date_before = chronoutil::delta::shift_months(scheduled_date, -1);

                    #[allow(clippy::if_same_then_else)]
                    if chrono::Local::today() >= scheduled_date
                        && last_run.end.date() < scheduled_date
                    {
                        Ok(DueCause::Regular)
                    } else if chrono::Local::today() >= scheduled_date_before
                        && last_run.end.date() < scheduled_date_before
                    {
                        Ok(DueCause::Regular)
                    } else {
                        let next = if chrono::Local::today() < scheduled_date {
                            scheduled_date
                        } else {
                            chronoutil::delta::shift_months(scheduled_date, 1)
                        };

                        Err(Self::NotDue {
                            next: next.and_hms(0, 0, 0),
                        })
                    }
                }
            }
        } else {
            // never ran before
            Ok(DueCause::Regular)
        }
    }
}

#[test]
fn test_check_running() {
    let config = config::Backup::test_new_mock();
    let mut history = config::history::History::default();
    let activity = config::Activity {
        used: super::USED_THRESHOLD,
        last_update: chrono::Local::now(),
    };

    history.running = Some(config::history::Running {
        start: chrono::Local::now(),
    });

    let due = Due::check_full(&config, Some(&history), Some(&activity));
    matches::assert_matches!(due, Err(Due::Running));
}

#[test]
fn test_check_daily() {
    let mut config = config::Backup::test_new_mock();
    let mut history = config::history::History::default();
    let activity = config::Activity {
        used: super::USED_THRESHOLD,
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
                (chrono::Local::now() + chrono::Duration::from_std(super::USED_THRESHOLD).unwrap())
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

    history.insert(config::history::RunInfo::new_left_running(
        &chrono::Local::now(),
    ));

    let due = Due::check_full(&config, Some(&history), Some(&activity));
    assert!(match due {
        Err(Due::NotDue { next }) => {
            assert_eq!(
                next,
                chrono::Local::today()
                    .succ()
                    .and_time(preferred_time)
                    .unwrap()
            );
            true
        }
        _ => false,
    });

    // completed today

    history.run = Default::default();
    history.insert(config::history::RunInfo::test_new_mock(
        chrono::Duration::zero(),
    ));

    let due = Due::check_full(&config, Some(&history), Some(&activity));
    assert!(match due {
        Err(Due::NotDue { next }) => {
            assert_eq!(
                next,
                chrono::Local::today()
                    .succ()
                    .and_time(preferred_time)
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
        used: super::USED_THRESHOLD,
        last_update: chrono::Local::now(),
    };

    config.schedule.frequency = config::Frequency::Weekly {
        preferred_weekday: (chrono::Local::today() - chrono::Duration::days(1)).weekday(),
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
                (chrono::Local::now() + chrono::Duration::from_std(super::USED_THRESHOLD).unwrap())
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
                (chrono::Local::today() + chrono::Duration::days(6)).and_hms(0, 0, 0)
            );
            true
        }
        _ => false,
    });

    // due today and only completed yesterday

    config.schedule.frequency = config::Frequency::Weekly {
        preferred_weekday: chrono::Local::today().weekday(),
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
                chrono::Local::today().and_hms(0, 0, 0) + chrono::Duration::weeks(1)
            );
            true
        }
        _ => false,
    });
}
