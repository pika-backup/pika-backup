use chrono::prelude::*;

use crate::borg;
use crate::borg::msg;
use crate::borg::Run;
use crate::config;
use crate::config::*;
use crate::ui::globals::*;
use crate::ui::prelude::*;

impl Display {
    pub fn new_from_id(config_id: &ConfigId) -> Self {
        if let Some(communication) = BACKUP_COMMUNICATION.load().get(config_id) {
            Self::from(communication)
        } else if let Some(BackupConfig {
            last_run: Some(backup),
            ..
        }) = SETTINGS.load().backups.get(config_id)
        {
            Self::from(backup)
        } else {
            Self::default()
        }
    }
}

impl From<&config::RunInfo> for Display {
    fn from(run_info: &config::RunInfo) -> Self {
        match &run_info.result {
            Ok(_) => Self {
                title: gettext("Last backup successful"),
                subtitle: Some(format!(
                    "About {}",
                    (run_info.end - Local::now()).humanize()
                )),
                graphic: Graphic::Icon("emblem-default-symbolic".to_string()),
                progress: None,
                run_info: Some(run_info.clone()),
                progress_archive: None,
            },
            Err(_) => Self {
                title: gettext("Last backup failed"),
                subtitle: Some(format!(
                    "About {}",
                    (run_info.end - Local::now()).humanize()
                )),
                graphic: Graphic::ErrorIcon("dialog-error-symbolic".to_string()),
                progress: None,
                run_info: Some(run_info.clone()),
                progress_archive: None,
            },
        }
    }
}

impl From<&borg::Communication> for Display {
    fn from(communication: &borg::Communication) -> Self {
        let status = communication.status.get();

        let mut progress = None;
        let mut progress_archive = None;
        let mut subtitle = None;

        if let Some(ref last_message) = status.last_message {
            match *last_message {
                msg::Progress::Archive(ref progress_archive_ref) => {
                    progress_archive = Some(progress_archive_ref.clone());
                    if let Some(total) = status.estimated_size {
                        let fraction = progress_archive_ref.original_size as f64 / total as f64;
                        progress = Some(fraction);

                        subtitle = Some(gettextf(
                            // xgettext:no-c-format
                            "{} % finished",
                            &[&format!("{:.1}", fraction * 100.0)],
                        ));
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
                    &[&crate::BORG_DELAY_RECONNECT.humanize()],
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
            run_info: None,
            progress_archive,
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
            run_info: None,
            progress_archive: None,
        }
    }
}

pub struct Display {
    pub title: String,
    pub subtitle: Option<String>,
    pub graphic: Graphic,
    pub progress: Option<f64>,
    pub run_info: Option<RunInfo>,
    pub progress_archive: Option<msg::ProgressArchive>,
}

pub enum Graphic {
    Icon(String),
    ErrorIcon(String),
    Spinner,
}
