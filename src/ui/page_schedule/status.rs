use crate::config;
use crate::daemon::schedule::requirements;
use crate::ui::prelude::*;
use crate::ui::utils::{StatusLevel, StatusRow};

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
    pub fn new(config: &config::Backup) -> Self {
        let due_requirements = requirements::Due::check(config);
        let global_requirements = requirements::Global::check(config);
        let hints = requirements::Hint::check(config);

        if !config.schedule.enabled {
            Status {
                main: StatusRow {
                    title: gettext("Scheduled backups disabled"),
                    subtitle: String::new(),
                    icon_name: String::from("x-office-calendar-symbolic"),
                    level: StatusLevel::Warning,
                },
                problems: vec![],
            }
        } else {
            let mut problem_level = StatusLevel::Error;

            let mut main_title = String::new();
            let mut main_subtitle = String::new();
            let mut main_level = StatusLevel::Ok;

            if let Err(due) = due_requirements {
                main_title = match config.schedule.frequency {
                    config::Frequency::Hourly => gettext("Hourly backups enabled"),
                    config::Frequency::Daily { .. } => gettext("Daily backups enabled"),
                    config::Frequency::Weekly { .. } => gettext("Weekly backups enabled"),
                    config::Frequency::Monthly { .. } => gettext("Monthly backups enabled"),
                };

                if let Some(scheduled_in) = &due.next_due() {
                    main_subtitle = next_backup_in(scheduled_in);

                    if !global_requirements.is_empty() || !hints.is_empty() {
                        // TODO proper format
                        main_subtitle += &format!(
                            " – {}",
                            gettext("Will not start until requirements are met.")
                        );
                    }
                } else {
                    if BACKUP_COMMUNICATION.load().get(&config.id).is_none() {
                        main_subtitle = gettext("Iconsitent backup information");
                        main_level = StatusLevel::Error;
                    }
                }

                problem_level = StatusLevel::Warning;
            } else if !global_requirements.is_empty() || !hints.is_empty() {
                main_title = gettext("Backup past due");
                main_subtitle = gettext("Waiting until requirements are met");
                main_level = StatusLevel::Warning;
            } else {
                main_title = gettext("Waiting for backup to start");
                main_level = StatusLevel::Error;
            }

            let mut problems = Vec::new();

            for problem in global_requirements {
                problems.push(StatusRow {
                    title: format!("{:?}", problem),
                    subtitle: String::new(),
                    icon_name: String::from("x"),
                    level: problem_level,
                });
            }

            for hint in hints {
                match hint {
                    requirements::Hint::DeviceMissing => problems.push(StatusRow {
                        title: gettext("Backup device needs to be connected"),
                        subtitle: String::new(),
                        icon_name: String::from("drive-removable-media-symbolic"),
                        level: problem_level,
                    }),
                }
            }

            Status {
                main: StatusRow {
                    title: main_title,
                    subtitle: main_subtitle,
                    icon_name: String::from("x-office-calendar-symbolic"),
                    level: main_level,
                },
                problems: problems,
            }
        }
    }
}
