use chrono::prelude::*;

use crate::borg;
use crate::borg::msg;
use crate::borg::Run;
use crate::config::history;
use crate::config::*;
use crate::ui::prelude::*;
use crate::ui::utils;

pub struct Display {
    pub title: String,
    pub subtitle: Option<String>,
    pub graphic: Graphic,
    pub progress: Option<f64>,
    pub stats: Option<Stats>,
}

pub enum Stats {
    Progress(msg::ProgressArchive),
    Final(history::RunInfo),
}

pub enum Graphic {
    Icon(String),
    WarningIcon(String),
    ErrorIcon(String),
    Spinner,
}

impl Display {
    pub fn new_from_id(config_id: &ConfigId) -> Self {
        if let Some(communication) = BACKUP_COMMUNICATION.load().get(config_id) {
            Self::from(communication)
        } else if let Some(last_run) = BACKUP_HISTORY
            .load()
            .get_result(config_id)
            .ok()
            .and_then(|x| x.run.get(0))
        {
            Self::from(last_run)
        } else {
            Self::default()
        }
    }
}

impl From<&history::RunInfo> for Display {
    fn from(run_info: &history::RunInfo) -> Self {
        match run_info.outcome {
            borg::Outcome::Completed { .. }
                if run_info.messages.max_log_level() > Some(borg::msg::LogLevel::Info) =>
            {
                Self {
                    title: gettext("Last backup completed with warnings"),
                    subtitle: Some(utils::duration::ago(&(Local::now() - run_info.end))),
                    graphic: Graphic::WarningIcon("dialog-warning-symbolic".to_string()),
                    progress: None,
                    stats: Some(Stats::Final(run_info.clone())),
                }
            }
            borg::Outcome::Completed { .. } => Self {
                title: gettext("Last backup successful"),
                subtitle: Some(utils::duration::ago(&(Local::now() - run_info.end))),
                graphic: Graphic::Icon("emblem-default-symbolic".to_string()),
                progress: None,
                stats: Some(Stats::Final(run_info.clone())),
            },
            _ => Self {
                title: gettext("Last backup failed"),
                subtitle: Some(utils::duration::ago(&(Local::now() - run_info.end))),
                graphic: Graphic::ErrorIcon("dialog-error-symbolic".to_string()),
                progress: None,
                stats: Some(Stats::Final(run_info.clone())),
            },
        }
    }
}

impl From<&borg::Communication> for Display {
    fn from(communication: &borg::Communication) -> Self {
        let status = communication.status.get();

        let mut progress = None;
        let mut stats = None;
        let mut subtitle = None;

        if let Some(ref last_message) = status.last_message {
            match *last_message {
                msg::Progress::Archive(ref progress_archive_ref) => {
                    stats = Some(Stats::Progress(progress_archive_ref.clone()));
                    if let Some(size) = &status.estimated_size {
                        let fraction =
                            progress_archive_ref.original_size as f64 / size.total as f64;
                        progress = Some(fraction);

                        let mut sub = gettextf(
                            // xgettext:no-c-format
                            "{} % finished",
                            &[&format!("{:.1}", fraction * 100.0)],
                        );

                        if let Some(duration) = status.time_remaining() {
                            sub.push_str(&format!(" – {}", utils::duration::left(&duration)));
                        }

                        subtitle = Some(sub);
                    }
                }
                msg::Progress::Message {
                    message: Some(ref message),
                    ref msgid,
                    ..
                } => {
                    if msgid.as_ref().map(|x| x.starts_with("cache.")) == Some(true) {
                        subtitle = Some(gettext("Updating repository information"));
                    } else {
                        subtitle = Some(message.clone());
                    }
                }
                msg::Progress::Percent {
                    current: Some(current),
                    total: Some(total),
                    ..
                } => {
                    let fraction = current as f64 / total as f64;
                    progress = Some(fraction);
                    subtitle = Some(gettextf(
                        // xgettext:no-c-format
                        "{} % prepared",
                        &[&format!("{:.1}", fraction * 100.0)],
                    ))
                }
                // TODO: cover progress message?
                _ => {}
            }
        }

        let title = match status.run {
            Run::Init => gettext("Preparing backup"),
            Run::SizeEstimation => gettext("Estimating backup size"),
            Run::Running => gettext("Backup running"),
            Run::Reconnecting => {
                subtitle = Some(gettextf(
                    "Connection lost, reconnecting in {}",
                    &[&utils::duration::plain(&utils::duration::from_std(
                        crate::BORG_DELAY_RECONNECT,
                    ))],
                ));
                gettext("Reconnecting")
            }
            Run::Stopping => gettext("Stopping backup"),
        };

        Self {
            title,
            subtitle,
            graphic: Graphic::Spinner,
            progress,
            stats,
        }
    }
}

impl Default for Display {
    fn default() -> Self {
        Self {
            title: gettext("Backup never ran"),
            subtitle: Some(gettext("Start by creating your first backup")),
            graphic: Graphic::Icon("dialog-information-symbolic".to_string()),
            progress: None,
            stats: None,
        }
    }
}
