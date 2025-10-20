use std::fmt::Write;

use common::config;
use common::schedule::requirements;

use crate::prelude::*;
use crate::utils::StatusLevel;
use crate::widget::StatusRow;

pub struct Status {
    pub main: StatusRow,
    pub problems: Vec<StatusRow>,
}

pub fn next_backup_in(d: &chrono::Duration) -> String {
    if d.num_hours() < 1 {
        ngettextf_(
            "Next backup in one minute",
            "Next backup in {} minutes",
            d.num_minutes() as u32,
        )
    } else if d.num_days() < 1 {
        ngettextf_(
            "Next backup in one hour",
            "Next backup in {} hours",
            d.num_hours() as u32,
        )
    } else if d.num_weeks() < 1 {
        ngettextf_(
            "Next backup in one day",
            "Next backup in {} days",
            d.num_days() as u32,
        )
    } else {
        ngettextf_(
            "Next backup in one week",
            "Next backup in {} weeks",
            d.num_weeks() as u32,
        )
    }
}

impl Status {
    pub async fn new(config: &config::Backup) -> Self {
        let due_requirements = requirements::Due::check(config, None, None);
        let global_requirements = requirements::Global::check(
            config,
            BACKUP_CONFIG.load().as_ref(),
            BACKUP_HISTORY.load().as_ref(),
        )
        .await;
        let hints = requirements::Hint::check(config);

        if !config.schedule.enabled {
            Self {
                main: StatusRow::new(
                    gettext("Scheduled Backups Disabled"),
                    "",
                    "schedule-symbolic",
                    StatusLevel::Warning,
                ),
                problems: vec![],
            }
        } else {
            let mut problem_level = StatusLevel::Error;

            let main_title;
            let mut main_subtitle = String::new();
            let mut main_level = StatusLevel::Ok;

            let mut upcoming_requirements_not_met = false;

            if let Err(due) = due_requirements {
                main_title = match config.schedule.frequency {
                    config::Frequency::Hourly => gettext("Hourly Backups Enabled"),
                    config::Frequency::Daily { .. } => gettext("Daily Backups Enabled"),
                    config::Frequency::Weekly { .. } => gettext("Weekly Backups Enabled"),
                    config::Frequency::Monthly { .. } => gettext("Monthly Backups Enabled"),
                };

                if let Some(scheduled_in) = &due.next_due() {
                    main_subtitle = next_backup_in(scheduled_in);

                    if !global_requirements.is_empty() || !hints.is_empty() {
                        // TODO proper format
                        let _ = write!(
                            main_subtitle,
                            " – {}",
                            gettext("Will not start until requirements are met.")
                        );
                        upcoming_requirements_not_met = true;
                    }
                } else if BORG_OPERATION.with(|op| op.load().get(&config.id).is_none()) {
                    main_subtitle = gettext("Inconsistent backup information");
                    main_level = StatusLevel::Error;
                }

                problem_level = StatusLevel::Warning;
            } else if !global_requirements.is_empty() || !hints.is_empty() {
                main_title = gettext("Backup Past Due");
                main_subtitle = gettext("Waiting until requirements are met");
                main_level = StatusLevel::Warning;
            } else if !status_tracking().daemon_running.get() {
                main_title = gettext("Scheduled Backups Unavailable");
                main_level = StatusLevel::Error;
            } else {
                main_title = gettext("Waiting for Backup to Start");
                main_level = StatusLevel::Error;
            }

            let mut problems = Vec::new();

            for problem in global_requirements {
                match problem {
                    requirements::Global::MeteredConnection => problems.push(StatusRow::new(
                        gettext("Network connection must not be metered"),
                        "",
                        "money-symbolic",
                        problem_level,
                    )),
                    requirements::Global::OtherBackupRunning(_) => problems.push(StatusRow::new(
                        gettext("Other backups on repository have to be completed"),
                        "",
                        "media-playback-start-symbolic",
                        problem_level,
                    )),
                    requirements::Global::Browsing => problems.push(StatusRow::new(
                        gettext("Archives cannot be opened for browsing"),
                        "",
                        "folder-open-symbolic",
                        problem_level,
                    )),
                    requirements::Global::ThisBackupRunning => (),
                    requirements::Global::OnBattery => problems.push(StatusRow::new(
                        gettext("Device must be connected to power"),
                        "",
                        "battery-good-symbolic",
                        problem_level,
                    )),
                }
            }

            for hint in hints {
                match hint {
                    requirements::Hint::DeviceMissing => problems.push(StatusRow::new(
                        gettext("Backup device has to be connected"),
                        if upcoming_requirements_not_met {
                            gettext("Reminder will be sent when device is required")
                        } else {
                            "".to_string()
                        },
                        "drive-removable-media-symbolic",
                        problem_level,
                    )),
                    requirements::Hint::NetworkMissing => problems.push(StatusRow::new(
                        gettext("Network connection has to be available"),
                        "",
                        "network-offline-symbolic",
                        problem_level,
                    )),
                }
            }

            if !status_tracking().daemon_running.get() {
                problems.push(StatusRow::new(
                    gettext("Background process inactive"),
                    gettext("This is required for scheduled backups"),
                    "action-unavailable-symbolic",
                    StatusLevel::Error,
                ));
            }

            Self {
                main: StatusRow::new(main_title, main_subtitle, "schedule-symbolic", main_level),
                problems,
            }
        }
    }
}
