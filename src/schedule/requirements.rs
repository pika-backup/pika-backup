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

use std::collections::BTreeMap;

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
    Browsing,
    /// May not use metered connection
    MeteredConnection,
    OnBattery,
}

impl Global {
    /// Returns all requirements that are violated
    pub async fn check(config: &config::Backup, histories: &config::Histories) -> Vec<Self> {
        let mut vec = Vec::new();
        let settings = &config.schedule.settings;

        let histories_with_repo_id = histories
            .iter()
            .filter(|(config_id, _)| {
                backup_config().try_get(config_id).map(|x| &x.repo_id) == Ok(&config.repo_id)
            })
            .collect::<BTreeMap<_, _>>();

        let running_backup = histories_with_repo_id
            .iter()
            .find(|(_, history)| history.is_running());

        if let Some((running_config_id, _)) = running_backup {
            // TODO: Is this ever triggered?
            if **running_config_id == config.id {
                vec.push(Self::ThisBackupRunning)
            } else {
                vec.push(Self::OtherBackupRunning((*running_config_id).clone()))
            }
        }

        if histories_with_repo_id
            .iter()
            .any(|(_, history)| history.is_browsing())
        {
            vec.push(Self::Browsing)
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
pub enum Due<Tz: chrono::TimeZone> {
    NotDue { next: DateTime<Tz> },
    Running,
}

#[derive(Debug, Clone, Serialize, Deserialize, zbus::zvariant::Type)]
pub enum DueCause {
    Regular,
    Retry,
}

impl Due<Local> {
    pub fn next_due(&self) -> Option<chrono::Duration> {
        match self {
            Self::NotDue { next } => Some(next.with_timezone(&Local) - Local::now()),
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
            Local::now(),
        )
    }
}

impl<Tz: chrono::TimeZone> Due<Tz> {
    /// Same as check_full but does not rely on the current system time.
    ///
    /// Checks all the prerequisites in the history file and then delegates the
    /// actual implementation to [`check_real`] to allow for more
    /// comprehensive testing.
    fn check_with_frequency(
        schedule: config::Frequency,
        history: Option<&config::history::History>,
        activity: &config::Activity,
        now: chrono::DateTime<Tz>,
    ) -> Result<DueCause, Self> {
        if history.is_some_and(|h| h.is_running()) {
            // Already running, skip
            return Err(Self::Running);
        };

        // Convert all times to the timezone of now
        let tz = now.timezone();

        // The last backup, no matter if successful or not
        let Some(last_run) = history
            .and_then(|h| h.last_run())
            .map(|run| run.end.with_timezone(&tz))
        else {
            // Never ran before, always due
            return Ok(DueCause::Regular);
        };

        // The last successful backup
        let last_completed = history
            .and_then(|h| h.last_completed())
            .map(|run| run.end.with_timezone(&tz));

        Self::check_real(schedule, activity, now, last_run, last_completed)
    }

    /// The actual schedule calculation takes place here
    fn check_real(
        frequency: config::Frequency,
        activity: &config::Activity,
        now: chrono::DateTime<Tz>,
        last_run: chrono::DateTime<Tz>,
        last_completed: Option<chrono::DateTime<Tz>>,
    ) -> Result<DueCause, Self> {
        // The output timezone
        let tz = now.timezone();

        // Convert the last run into naive date time
        let last_run_naive = last_run.naive_local();
        let last_completed_naive = last_completed.map(|c| c.naive_local());

        // The next backup according to the regular schedule
        let next_run =
            Self::naive_to_next_local(Self::next_run(frequency, last_run_naive), tz.clone());

        // Check if we are due for a regular backup
        if next_run <= now {
            // Check if the device has been in use for at least USED_THRESHOLD minutes. We
            // ignore this for hourly.
            if matches!(frequency, config::Frequency::Hourly) || activity.is_threshold_reached() {
                return Ok(DueCause::Regular);
            } else {
                // We will be due soon, just wait a little longer until the device has been in
                // use for a few more minutes
                return Err(Self::NotDue {
                    next: now + activity.time_until_threshold(),
                });
            }
        }

        // We are not technically due. We might however be eligible for a retry.
        //
        // Retries work on weekly and monthly backups. The idea is that if the last
        // scheduled backup failed we try again the day after to ensure a valid
        // backup every week / month.
        match frequency {
            config::Frequency::Weekly { .. } | config::Frequency::Monthly { .. } => {
                // The next day after the last run, 00:00 (start of day)
                let next_day_midnight =
                    (last_run_naive.date() + chrono::Days::new(1)).and_time(chrono::NaiveTime::MIN);

                // We only allow one retry a day
                let next_retry = Self::naive_to_next_local(
                    if let Some(completed) = last_completed_naive {
                        // We use the time for the last completed backup to figure out the next
                        // retry the same way as we would with a regular
                        // scheduled backup.
                        Self::next_run(frequency, completed).max(next_day_midnight)
                    } else {
                        // If we never completed a backup we try every day until we get one.
                        next_day_midnight
                    },
                    now.timezone(),
                );

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

    /// Calculate the next date that exists.
    ///
    /// This deals with gaps in the space-time continuum by finding the next
    /// representable time after the gap.
    fn naive_to_next_local(mut naive: chrono::NaiveDateTime, tz: Tz) -> chrono::DateTime<Tz> {
        loop {
            let next = naive.and_local_timezone(tz.clone()).earliest();
            if let Some(next) = next {
                return next;
            } else {
                naive += chrono::Duration::minutes(1);
            }
        }
    }

    /// Determine the next scheduled backup time.
    ///
    /// This returns the time as a naive date time. Converting this to the local
    /// time is left to the caller.
    fn next_run(frequency: config::Frequency, last_run: NaiveDateTime) -> NaiveDateTime {
        match frequency {
            // Hourly backups just run every hour (measured after the last run end time)
            config::Frequency::Hourly => last_run + chrono::Duration::hours(1),
            // Daily backups run every day at the preferred time or later
            config::Frequency::Daily { preferred_time } => {
                // First we change the date if needed
                let next_date = if last_run.time() < preferred_time {
                    // Schedule for the same day because we ran before the preferred time
                    last_run.date()
                } else {
                    // We already ran today on or after the scheduled time. Schedule for the next
                    // day.
                    last_run.date() + chrono::Duration::days(1)
                };

                // Now we adjust the time to our preferred time.
                next_date.and_time(preferred_time)
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
                let next_run = last_run.date() + chrono::Days::new(offset_days.into());
                next_run.and_time(chrono::NaiveTime::MIN)
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
                    last_run.date() + chrono::Months::new(1)
                } else {
                    // We run this month
                    last_run.date()
                };

                // Set the day. This will clamp to the last day of the month if the month is
                // shorter.
                //
                // Panics: This only fails at the end of time
                chronoutil::delta::with_day(
                    next_month.and_time(chrono::NaiveTime::MIN),
                    preferred_day,
                )
                .unwrap()
            }
        }
    }
}

#[cfg(test)]
mod test {
    use config::{Activity, Frequency};
    use matches::assert_matches;

    use super::*;
    use crate::schedule::USED_THRESHOLD;

    #[test]
    fn test_check_running() {
        let config = config::Backup::test_new_mock();
        let mut history = config::history::History::default();
        let activity = Activity {
            used: USED_THRESHOLD,
            last_update: Local::now(),
        };

        history.start_running_now();

        let due = Due::check_full(&config, Some(&history), Some(&activity));
        matches::assert_matches!(due, Err(Due::Running));
    }

    #[test]
    fn test_check_real_hourly() {
        let now = Local.with_ymd_and_hms(2024, 02, 12, 13, 02, 12).unwrap();
        let yesterday = now.with_day(11).unwrap();
        let tomorrow = now.with_day(13).unwrap();
        let last_month = Local.with_ymd_and_hms(2024, 01, 31, 12, 59, 12).unwrap();
        let activity_none = Activity::default();
        let activity_enough = Activity {
            used: USED_THRESHOLD,
            last_update: Local::now(),
        };

        // Activity or completed shouldn't matter for hourly
        for activity in [activity_none, activity_enough] {
            for completed in [None, Some(yesterday), Some(now), Some(tomorrow)] {
                debug!("activity: {:?}, completed: {:?}", activity, completed);

                // Yesterday
                assert_matches!(
                    Due::check_real(Frequency::Hourly, &activity, now, yesterday, completed),
                    Ok(DueCause::Regular)
                );

                // Last month
                assert_matches!(
                    Due::check_real(Frequency::Hourly, &activity, now, last_month, completed),
                    Ok(DueCause::Regular)
                );

                // Exactly one hour earlier
                assert_matches!(
                    Due::check_real(
                        Frequency::Hourly,
                        &activity,
                        now,
                        Local.with_ymd_and_hms(2024, 02, 12, 12, 02, 12).unwrap(),
                        completed
                    ),
                    Ok(DueCause::Regular)
                );

                // Exactly one hour and one second earlier
                assert_matches!(
                    Due::check_real(
                        Frequency::Hourly,
                        &activity,
                        now,
                        Local.with_ymd_and_hms(2024, 02, 12, 12, 02, 11).unwrap(),
                        completed
                    ),
                    Ok(DueCause::Regular)
                );

                // Exactly 59 minutes 59 seconds earlier
                let next_schedule = Local.with_ymd_and_hms(2024, 02, 12, 13, 02, 13).unwrap();
                assert_matches!(
                    Due::check_real(
                        Frequency::Hourly,
                        &activity,
                        now,
                        Local
                            .with_ymd_and_hms(2024, 02, 12, 12, 02, 13)
                            .unwrap(),
                            completed
                    ),
                    Err(Due::NotDue { next }) if next == next_schedule
                );

                // Next backup is exactly one hour after the last run, even if the last run is
                // in the future. The algorithm should not depend on system
                // time.
                assert_matches!(
                    Due::check_real(
                        Frequency::Hourly,
                        &activity,
                        now,
                        tomorrow,
                        completed
                    ),
                    Err(Due::NotDue { next }) if next == tomorrow + chrono::Duration::hours(1)
                );
            }
        }
    }

    #[test]
    fn test_check_real_daily() {
        let now = Local.with_ymd_and_hms(2024, 02, 12, 13, 02, 12).unwrap();
        let preferred_time = chrono::NaiveTime::from_hms_opt(11, 02, 01).unwrap();
        let yesterday = now.with_day(11).unwrap();
        let tomorrow = now.with_day(13).unwrap();
        let last_month = Local.with_ymd_and_hms(2024, 01, 31, 12, 59, 12).unwrap();
        let activity_none = Activity::default();
        let activity_enough = Activity {
            used: USED_THRESHOLD,
            last_update: Local::now(),
        };

        // Completed shouldn't matter for daily, we don't do retries for daily
        for completed in [None, Some(yesterday), Some(now), Some(tomorrow)] {
            for date in [now, last_month, yesterday, tomorrow] {
                for last_date in [now, last_month, yesterday, tomorrow] {
                    println!(
                        "date: {:?}, last_date: {:?}, completed: {:?}",
                        date, last_date, completed
                    );

                    // Run at least USED_THRESHOLD after "now" if activity is empty no matter what
                    // time we ran the last time
                    assert_matches!(
                        Due::check_real(
                            Frequency::Daily { preferred_time },
                            &activity_none,
                            date,
                            last_date,
                            completed
                        ),
                        Err(Due::NotDue { next }) if next >= date + USED_THRESHOLD
                    )
                }
            }

            // We ran yesterday, so we should run again now
            assert_matches!(
                Due::check_real(
                    Frequency::Daily { preferred_time },
                    &activity_enough,
                    now,
                    yesterday,
                    completed
                ),
                Ok(DueCause::Regular)
            );

            // We ran last month, so we should run again now
            assert_matches!(
                Due::check_real(
                    Frequency::Daily { preferred_time },
                    &activity_enough,
                    now,
                    last_month,
                    completed
                ),
                Ok(DueCause::Regular)
            );

            // We ran an hour ago. We shouldn't run again until tomorrow at the preferred
            // time.
            assert_matches!(
                Due::check_real(
                    Frequency::Daily { preferred_time },
                    &activity_enough,
                    now,
                    Local
                        .with_ymd_and_hms(2024, 02, 12, 12, 02, 12)
                        .unwrap(),
                    completed
                ),
                Err(Due::NotDue { next }) if next == tomorrow.with_time(preferred_time).unwrap()
            );

            // We finished two seconds before the preferred time. Now it's an hour later.
            // The last backup technically ran before "today at the preferred time" so we
            // run again. TODO: This is probably not ideal, do we want to
            // introduce a "grace period" of couple minutes?
            assert_matches!(
                Due::check_real(
                    Frequency::Daily { preferred_time },
                    &activity_enough,
                    now,
                    now.with_time(preferred_time - chrono::Duration::seconds(2))
                        .unwrap(),
                    completed
                ),
                Ok(DueCause::Regular)
            );

            // Same as above but the backup finished 2 seconds ago. We are still due.
            assert_matches!(
                Due::check_real(
                    Frequency::Daily { preferred_time },
                    &activity_enough,
                    now.with_time(preferred_time).unwrap(),
                    now.with_time(preferred_time - chrono::Duration::seconds(2))
                        .unwrap(),
                    completed
                ),
                Ok(DueCause::Regular)
            );

            // The backup ran 1 second ago, which was one second before preferred time.
            // we are one second before preferred time, we schedule a backup to run in one
            // second.
            assert_matches!(
                Due::check_real(
                    Frequency::Daily { preferred_time },
                    &activity_enough,
                    now.with_time(preferred_time - chrono::Duration::seconds(1))
                        .unwrap(),
                    now.with_time(preferred_time - chrono::Duration::seconds(2))
                        .unwrap(),
                    completed
                ),
                Err(Due::NotDue { next }) if next == now.with_time(preferred_time).unwrap()
            );
        }
    }

    #[test]
    fn test_check_real_weekly() {
        let now_monday = Local.with_ymd_and_hms(2024, 02, 12, 13, 02, 12).unwrap();
        let tomorrow = now_monday.with_day(13).unwrap();
        let preferred_weekday = chrono::Weekday::Sat;
        let last_sunday = now_monday.with_day(11).unwrap();
        let last_friday = now_monday.with_day(9).unwrap();
        let next_saturday_00 = now_monday
            .with_time(chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap())
            .unwrap()
            .with_day(17)
            .unwrap();
        let last_month = Local.with_ymd_and_hms(2024, 01, 31, 12, 59, 12).unwrap();
        let activity_none = Activity::default();
        let activity_enough = Activity {
            used: USED_THRESHOLD,
            last_update: Local::now(),
        };

        for date in [now_monday, last_month, last_sunday, next_saturday_00] {
            for last_date in [now_monday, last_month, last_sunday, next_saturday_00] {
                // the exact time of day shouldn't matter for weekly
                for hour in 0..13 {
                    // The weekday is irrelevant for this test
                    let mut weekday = preferred_weekday;
                    for _ in 0..7 {
                        weekday = weekday.succ();
                        let dt = date.with_hour(hour).unwrap();
                        println!("date: {:?}, last_date: {:?}, hour: {}", dt, last_date, hour);

                        // Run at least USED_THRESHOLD after "now" if activity is empty no matter
                        // what time we ran the last time
                        assert_matches!(
                            Due::check_real(
                                Frequency::Weekly { preferred_weekday: weekday },
                                &activity_none,
                                dt,
                                last_date,
                                None
                            ),
                            Err(Due::NotDue { next }) if next >= dt + USED_THRESHOLD
                        )
                    }
                }
            }
        }

        // We ran saturday, now it's monday, our preferred day is saturday.
        // Schedule for next saturday at 00:00
        assert_matches!(
            Due::check_real(
                Frequency::Weekly { preferred_weekday },
                &activity_enough,
                now_monday,
                last_sunday,
                Some(last_sunday)
            ),
            Err(Due::NotDue { next }) if next == Local
                .with_ymd_and_hms(2024, 02, 17, 0, 0, 0)
                .unwrap()
        );

        // We ran saturday but the backup didn't complete. The last complete backup was
        // on friday. Now it's monday. We are due for a retry.
        assert_matches!(
            Due::check_real(
                Frequency::Weekly { preferred_weekday },
                &activity_enough,
                now_monday,
                last_sunday,
                Some(last_friday)
            ),
            Ok(DueCause::Retry)
        );

        // We ran saturday but the backup didn't complete. The last complete backup was
        // on friday. Now it's monday. We already ran a retry today at 3 am. Our
        // next retry is scheduled for tomorrow.
        assert_matches!(
            Due::check_real(
                Frequency::Weekly { preferred_weekday },
                &activity_enough,
                now_monday,
                now_monday
                    .with_time(chrono::NaiveTime::from_hms_opt(3, 0, 0).unwrap())
                    .unwrap(),
                Some(last_friday)
            ),
            Err(Due::NotDue { next }) if next == tomorrow.with_time(chrono::NaiveTime::MIN).unwrap()
        );

        // We last ran last month. We are due no matter what day it is.
        assert_matches!(
            Due::check_real(
                Frequency::Weekly { preferred_weekday },
                &activity_enough,
                now_monday,
                last_month,
                Some(last_month)
            ),
            Ok(DueCause::Regular)
        );
    }

    #[test]
    fn test_check_real_monthly() {
        let feb_12 = Local.with_ymd_and_hms(2024, 02, 12, 13, 02, 12).unwrap();
        let preferred_day = 14;
        let feb_14 = feb_12.with_day(14).unwrap();
        let feb_15 = feb_12.with_day(15).unwrap();
        let feb_29_00 = Local.with_ymd_and_hms(2024, 02, 29, 0, 0, 0).unwrap();
        let last_month = Local.with_ymd_and_hms(2024, 01, 31, 12, 59, 12).unwrap();
        let activity_none = Activity::default();
        let activity_enough = Activity {
            used: USED_THRESHOLD,
            last_update: Local::now(),
        };

        // Run at least USED_THRESHOLD after "now" if activity is empty no matter what
        // time we ran the last time
        for date in [feb_12, last_month, feb_14, feb_15, feb_29_00] {
            for last_date in [feb_12, last_month, feb_14, feb_15, feb_29_00] {
                // the exact time of day shouldn't matter for monthly
                for hour in 0..13 {
                    let dt = date.with_hour(hour).unwrap();
                    println!("date: {:?}, last_date: {:?}", dt, last_date);
                    assert_matches!(
                        Due::check_real(
                            Frequency::Monthly { preferred_day },
                            &activity_none,
                            dt,
                            last_date,
                            None
                        ),
                        Err(Due::NotDue { next }) if next >= dt + USED_THRESHOLD
                    )
                }
            }
        }

        // Our last backup was last month, and we have not yet passed the preferred day
        assert_matches!(
            Due::check_real(
                Frequency::Monthly { preferred_day },
                &activity_enough,
                feb_12,
                last_month,
                Some(last_month)
            ),
            Err(Due::NotDue { next }) if next == feb_14.with_time(chrono::NaiveTime::MIN).unwrap()
        );

        // Our last backup was last month, and today is the preferred day
        assert_matches!(
            Due::check_real(
                Frequency::Monthly { preferred_day },
                &activity_enough,
                feb_15,
                last_month,
                Some(last_month)
            ),
            Ok(DueCause::Regular)
        );

        // Our last backup was yesterday on the preferred day. But the backup failed.
        // We are due for a retry
        assert_matches!(
            Due::check_real(
                Frequency::Monthly { preferred_day },
                &activity_enough,
                feb_15,
                feb_14,
                Some(last_month)
            ),
            Ok(DueCause::Retry)
        );

        // Our preferred day is the 31st. Make sure we still schedule a backup in months
        // that don't have a 31st.
        assert_matches!(
            Due::check_real(
                Frequency::Monthly { preferred_day: 31 },
                &activity_enough,
                feb_15,
                feb_14,
                Some(last_month)
            ),
            Err(Due::NotDue {
                next
            }) if next == feb_29_00
        );
    }

    #[test]
    fn test_check_daily() {
        let mut config = config::Backup::test_new_mock();
        let mut history = config::history::History::default();
        let activity = config::Activity {
            used: USED_THRESHOLD,
            last_update: Local::now(),
        };
        let preferred_time = Local::now().time() - chrono::Duration::hours(1);

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
                    (Local::now() + chrono::Duration::from_std(USED_THRESHOLD).unwrap()) - next
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
            &(Local::now() - chrono::Duration::hours(2)),
        ));

        let due = Due::check_full(&config, Some(&history), Some(&activity));
        matches::assert_matches!(due, Ok(DueCause::Regular));

        // failed now, try again tomorrow

        let mut config_close = config;
        let preferred_time_close = Local::now().time() - chrono::Duration::seconds(1);

        let mut history_close = history.clone();
        history_close.insert(config::history::RunInfo::test_new_mock(
            chrono::Duration::seconds(1),
        ));

        config_close.schedule.frequency = config::Frequency::Daily {
            preferred_time: preferred_time_close,
        };

        history.insert(config::history::RunInfo::new_left_running(&Local::now()));

        let due = Due::check_full(&config_close, Some(&history_close), Some(&activity));
        assert!(match due {
            Err(Due::NotDue { next }) => {
                assert_eq!(
                    next,
                    Local::now()
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
                    Local::now()
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
            last_update: Local::now(),
        };

        config.schedule.frequency = config::Frequency::Weekly {
            preferred_weekday: (Local::now() - chrono::Duration::days(1)).weekday(),
        };

        // Never ran

        let due = Due::check_full(&config, Some(&history), Some(&activity));
        matches::assert_matches!(due, Ok(DueCause::Regular));

        // no activity

        history.insert(config::history::RunInfo::new_left_running(
            &(Local::now() - chrono::Duration::days(1)),
        ));

        let due = Due::check_full(&config, Some(&history), None);
        assert!(match due {
            Err(Due::NotDue { next }) => {
                // due after device used enough
                assert!(
                    (Local::now() + chrono::Duration::from_std(USED_THRESHOLD).unwrap()) - next
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
                    (Local::now() + chrono::Duration::days(6))
                        .with_time(chrono::NaiveTime::MIN)
                        .unwrap()
                );
                true
            }
            _ => false,
        });

        // due today and only completed yesterday

        config.schedule.frequency = config::Frequency::Weekly {
            preferred_weekday: Local::now().weekday(),
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
                    Local::now().with_time(chrono::NaiveTime::MIN).unwrap()
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
            last_update: Local::now(),
        };

        let preferred_day = Local::now() - chrono::Duration::days(1);
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

    /// Ensure we never schedule a backup in the past, even when directly at a
    /// time zone boundary
    #[test]
    fn schedule_in_the_past() {
        let tz = chrono_tz::Europe::Berlin;
        let last_run = tz
            .with_ymd_and_hms(2024, 10, 27, 0, 7, 0)
            .earliest()
            .unwrap()
            .with_timezone(&tz);

        let now = tz
            .with_ymd_and_hms(2024, 10, 27, 1, 30, 0)
            .earliest()
            .unwrap()
            .with_timezone(&tz);

        let frequency = config::Frequency::Daily {
            preferred_time: chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        };
        let activity = Activity::default();

        let due = Due::check_real(frequency, &activity, now, last_run, Some(last_run));
        assert!(match due {
            Err(Due::NotDue { next }) => {
                assert_eq!(
                    next,
                    tz.with_ymd_and_hms(2024, 10, 28, 0, 0, 0)
                        .earliest()
                        .unwrap()
                );
                true
            }
            _ => false,
        });
    }

    /// Ensure traveling between timezones works well
    #[test]
    fn tz_change() {
        let tz_berlin = chrono_tz::Europe::Berlin;
        let tz_nyc = chrono_tz::America::New_York;

        let last_run = tz_berlin
            .with_ymd_and_hms(2024, 10, 02, 0, 7, 0)
            .earliest()
            .unwrap()
            .with_timezone(&tz_nyc);

        let now = tz_nyc
            .with_ymd_and_hms(2024, 10, 01, 23, 30, 0)
            .earliest()
            .unwrap();

        let frequency = config::Frequency::Daily {
            preferred_time: chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        };
        let activity = Activity::default();

        let due = Due::check_real(frequency, &activity, now, last_run, Some(last_run));
        assert!(match due {
            Err(Due::NotDue { next }) => {
                assert_eq!(next.timezone(), tz_nyc);
                assert_eq!(
                    next,
                    tz_nyc
                        .with_ymd_and_hms(2024, 10, 02, 0, 0, 0)
                        .earliest()
                        .unwrap()
                );
                true
            }
            _ => false,
        });
    }

    /// Ensure gaps in the space-time contiuum are handled properly
    #[test]
    fn tz_gap() {
        let tz = chrono_tz::Europe::Berlin;
        let last_run = tz
            .with_ymd_and_hms(2024, 3, 30, 2, 30, 0)
            .earliest()
            .unwrap()
            .with_timezone(&tz);

        let now = tz
            .with_ymd_and_hms(2024, 3, 31, 0, 30, 0)
            .earliest()
            .unwrap()
            .with_timezone(&tz);

        let frequency = config::Frequency::Daily {
            preferred_time: chrono::NaiveTime::from_hms_opt(2, 0, 0).unwrap(),
        };
        let activity = Activity::default();

        // 2024-03-31 02:00 doesn't exist because of DST time change
        // We expect the next possible time: 03:00
        let due = Due::check_real(frequency, &activity, now, last_run, Some(last_run));
        assert!(match due {
            Err(Due::NotDue { next }) => {
                assert_eq!(
                    next,
                    tz.with_ymd_and_hms(2024, 3, 31, 3, 0, 0)
                        .earliest()
                        .unwrap()
                );
                true
            }
            _ => false,
        });
    }

    /// Ensure overlaps in the space-time contiuum are handled properly
    #[test]
    fn tz_overlap() {
        let tz = chrono_tz::Europe::Berlin;
        let last_run = tz
            .with_ymd_and_hms(2024, 10, 26, 2, 30, 0)
            .earliest()
            .unwrap()
            .with_timezone(&tz);

        let now = tz
            .with_ymd_and_hms(2024, 10, 27, 0, 30, 0)
            .earliest()
            .unwrap()
            .with_timezone(&tz);

        let frequency = config::Frequency::Daily {
            preferred_time: chrono::NaiveTime::from_hms_opt(2, 0, 0).unwrap(),
        };
        let activity = Activity::default();

        // 2024-03-31 02:00 exists twice because of DST time change
        // We expect this to be the first 02:00
        let due = Due::check_real(frequency, &activity, now, last_run, Some(last_run));
        assert!(match due {
            Err(Due::NotDue { next }) => {
                assert_eq!(
                    next,
                    tz.with_ymd_and_hms(2024, 10, 27, 2, 0, 0)
                        .earliest()
                        .unwrap()
                );
                true
            }
            _ => false,
        });
    }
}
