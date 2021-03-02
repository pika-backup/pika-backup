use crate::borg;
use crate::config;

use crate::prelude::*;
use chrono::prelude::*;
use std::collections::{BTreeMap, VecDeque};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct History {
    /// Last runs, latest run first
    pub run: VecDeque<RunInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Histories(pub BTreeMap<config::ConfigId, History>);

impl LookupConfigId<History> for crate::config::Histories {
    fn get_mut_result(
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
    pub fn from_default_path() -> std::io::Result<Self> {
        Ok(serde_json::de::from_reader(std::fs::File::open(
            Self::default_path()?,
        )?)?)
    }

    pub fn insert(&mut self, config_id: &ConfigId, entry: RunInfo) {
        if let Some(history) = self.0.get_mut(config_id) {
            history.run.push_front(entry);
        } else {
            let mut history: History = Default::default();
            history.run.push_front(entry);
            self.0.insert(config_id.clone(), history);
        }
    }

    pub fn default_path() -> std::io::Result<std::path::PathBuf> {
        crate::utils::prepare_config_file("history.json", Self::default())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RunInfo {
    pub end: DateTime<Local>,
    pub result: Result<borg::Stats, RunError>,
}

impl RunInfo {
    pub fn new(result: Result<borg::Stats, RunError>) -> Self {
        Self {
            end: Local::now(),
            result,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum RunError {
    WithLevel {
        message: String,
        level: borg::msg::LogLevel,
        stats: Option<borg::Stats>,
    },
    Simple(String),
}

impl RunError {
    pub fn level(&self) -> borg::msg::LogLevel {
        match self {
            Self::WithLevel { level, .. } => level.clone(),
            Self::Simple(_) => borg::msg::LogLevel::NONE,
        }
    }
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::WithLevel { message, .. } | Self::Simple(message) => message,
            }
        )
    }
}
