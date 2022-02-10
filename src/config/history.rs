use crate::borg;
use crate::config;

use super::Loadable;

use crate::prelude::*;
use chrono::prelude::*;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const HISTORY_LENGTH: usize = 100;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct History {
    /// Last runs, latest run first
    pub run: VecDeque<RunInfo>,
    pub running: Option<Running>,
    pub last_completed: Option<RunInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Histories(pub BTreeMap<config::ConfigId, History>);

impl super::ConfigType for Histories {
    fn path() -> std::path::PathBuf {
        let mut path = glib::user_config_dir();
        path.push(env!("CARGO_PKG_NAME"));
        path.push("history.json");

        path
    }
}

impl LookupConfigId for crate::config::Histories {
    type Item = History;

    fn get_result_mut(
        &mut self,
        key: &ConfigId,
    ) -> Result<&mut History, super::error::BackupNotFound> {
        self.0
            .get_mut(key)
            .ok_or_else(|| super::error::BackupNotFound::new(key.clone()))
    }

    fn get_result(&self, key: &ConfigId) -> Result<&History, super::error::BackupNotFound> {
        self.0
            .get(key)
            .ok_or_else(|| super::error::BackupNotFound::new(key.clone()))
    }
}

impl Histories {
    pub fn from_file_ui() -> std::io::Result<super::Writeable<Self>> {
        let mut histories: super::Writeable<Self> = super::Writeable::from_file()?;

        for (_, history) in histories.0.iter_mut() {
            if let Some(running) = &history.running {
                history
                    .run
                    .push_front(RunInfo::new_left_running(&running.start));
                history.running = None;
                history.run.truncate(HISTORY_LENGTH);
            }
        }

        Ok(histories)
    }

    pub fn insert(&mut self, config_id: ConfigId, entry: RunInfo) {
        let history = self.0.entry(config_id).or_default();

        if matches!(entry.outcome, borg::Outcome::Completed { .. }) {
            history.last_completed = Some(entry.clone());
        }

        history.running = None;
        history.run.push_front(entry);
        history.run.truncate(HISTORY_LENGTH);
    }

    pub fn set_running(&mut self, config_id: ConfigId) {
        debug!("Set {:?} to state running.", config_id);
        let history = self.0.entry(config_id).or_default();

        history.running = Some(Running {
            start: Local::now(),
        });
    }

    pub fn remove_running(&mut self, config_id: ConfigId) {
        let history = self.0.entry(config_id).or_default();

        history.running = None;
    }

    pub fn iter(&self) -> std::collections::btree_map::Iter<'_, config::ConfigId, History> {
        self.0.iter()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RunInfo {
    pub messages: borg::log_json::LogCollection,
    pub outcome: borg::Outcome,
    pub end: DateTime<Local>,
    pub include: BTreeSet<std::path::PathBuf>,
    pub exclude: BTreeSet<config::Pattern>,
}

impl RunInfo {
    pub fn new(
        config: &config::Backup,
        outcome: borg::Outcome,
        messages: borg::log_json::LogCollection,
    ) -> Self {
        Self {
            end: Local::now(),
            outcome,
            messages,
            include: config.include.clone(),
            exclude: config.exclude.clone(),
        }
    }

    pub fn new_left_running(date: &DateTime<Local>) -> Self {
        Self {
            end: *date,
            outcome: borg::Outcome::Aborted(borg::error::Abort::LeftRunning),
            messages: vec![],
            include: Default::default(),
            exclude: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Running {
    pub start: DateTime<Local>,
}
