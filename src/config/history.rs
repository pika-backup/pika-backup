use crate::borg;
use crate::borg::log_json::LogCollection;
use crate::config;

use super::Loadable;

use crate::prelude::*;
use chrono::prelude::*;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const HISTORY_LENGTH: usize = 100;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SuggestedExcludeReason {
    PermissionDenied,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct History {
    /// Configuration version
    #[serde(default)]
    config_version: super::Version,

    /// Last runs, latest run first
    run: VecDeque<RunInfo>,
    running: Option<Running>,
    last_completed: Option<RunInfo>,

    /// Last borg check result
    #[serde(default)]
    last_check: Option<CheckRunInfo>,

    // The excludes suggested from the last size estimate. Will be overwritten every time a size estimate is performed.
    #[serde(default)]
    suggested_exclude:
        BTreeMap<SuggestedExcludeReason, BTreeSet<config::Exclude<{ config::RELATIVE }>>>,
}

impl History {
    pub fn clear(&mut self) {
        *self = Default::default();
    }

    pub fn insert(&mut self, entry: RunInfo) {
        if matches!(entry.outcome, borg::Outcome::Completed { .. }) {
            self.last_completed = Some(entry.clone());
        }

        self.running = None;
        self.run.push_front(entry);
        self.run.truncate(HISTORY_LENGTH);
    }

    pub fn last_run(&self) -> Option<&RunInfo> {
        self.run.front()
    }

    pub fn start_running_now(&mut self) {
        self.running = Some(config::history::Running {
            start: chrono::Local::now(),
        });
    }

    pub fn is_running(&self) -> bool {
        self.running.is_some()
    }

    pub fn last_completed(&self) -> Option<&RunInfo> {
        self.last_completed.as_ref()
    }

    pub fn last_check(&self) -> Option<&CheckRunInfo> {
        self.last_check.as_ref()
    }

    pub fn set_suggested_excludes_from_absolute(
        &mut self,
        reason: SuggestedExcludeReason,
        paths: Vec<impl Into<std::path::PathBuf>>,
    ) {
        let mut excludes = BTreeSet::new();

        // Limit to 20 elements to not overfill the config with paths
        for path in paths.into_iter().take(20) {
            let pattern = config::Pattern::<{ config::RELATIVE }>::path_full_match(path);
            excludes.insert(config::Exclude::from_pattern(pattern));
        }

        // Overwrite the previous suggested exclude list
        self.suggested_exclude.insert(reason, excludes);
    }

    pub fn suggested_excludes_with_reason(
        &self,
        reason: SuggestedExcludeReason,
    ) -> Option<&BTreeSet<config::Exclude<{ config::RELATIVE }>>> {
        self.suggested_exclude.get(&reason)
    }
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

impl super::ConfigVersion for Histories {
    fn extract_version(json: &serde_json::Value) -> u64 {
        json.as_object()
            .and_then(|d| d.values().next())
            .and_then(|v| v.get("config_version"))
            .and_then(|v| v.as_u64())
            .unwrap_or(2)
    }
}

impl LookupConfigId for crate::config::Histories {
    type Item = History;

    fn try_get_mut(
        &mut self,
        key: &ConfigId,
    ) -> Result<&mut History, super::error::BackupNotFound> {
        self.0
            .get_mut(key)
            .ok_or_else(|| super::error::BackupNotFound::new(key.clone()))
    }

    fn try_get(&self, key: &ConfigId) -> Result<&History, super::error::BackupNotFound> {
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

    pub fn handle_shutdown(histories: &mut Self) {
        for (_, history) in histories.0.iter_mut() {
            if let Some(running) = &history.running {
                history
                    .run
                    .push_front(RunInfo::new_shutdown(&running.start));
                history.running = None;
                history.run.truncate(HISTORY_LENGTH);
            }
        }
    }

    pub fn insert(&mut self, config_id: ConfigId, entry: RunInfo) {
        let history = self.0.entry(config_id).or_default();

        history.insert(entry);
    }

    pub fn set_last_check(&mut self, config_id: ConfigId, check_info: CheckRunInfo) {
        let history = self.0.entry(config_id).or_default();

        history.last_check = Some(check_info);
    }

    pub fn set_running(&mut self, config_id: ConfigId) {
        debug!("Set {:?} to state running.", config_id);
        let history = self.0.entry(config_id).or_default();

        history.running = Some(Running {
            start: Local::now(),
        });
    }

    pub fn remove_running(&mut self, config_id: ConfigId) {
        debug!("Set {:?} to state not running", config_id);
        let history = self.0.entry(config_id).or_default();

        history.running = None;
    }

    pub fn iter(&self) -> std::collections::btree_map::Iter<'_, config::ConfigId, History> {
        self.0.iter()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RunInfo {
    pub end: DateTime<Local>,
    pub outcome: borg::Outcome,
    pub messages: borg::log_json::LogCollection,
    pub include: BTreeSet<std::path::PathBuf>,
    pub exclude: BTreeSet<config::Exclude<{ config::ABSOLUTE }>>,
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
            exclude: BTreeSet::from_iter(
                config
                    .exclude
                    .clone()
                    .into_iter()
                    .map(|x| x.into_absolute()),
            ),
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

    pub fn new_shutdown(date: &DateTime<Local>) -> Self {
        Self {
            end: *date,
            outcome: borg::Outcome::Aborted(borg::error::Abort::Shutdown),
            messages: vec![],
            include: Default::default(),
            exclude: Default::default(),
        }
    }

    #[cfg(test)]
    pub fn test_new_mock(ago: chrono::Duration) -> Self {
        Self {
            end: Local::now() - ago,
            outcome: borg::Outcome::Completed {
                stats: borg::json::Stats::test_new_mock(),
            },
            messages: Default::default(),
            include: Default::default(),
            exclude: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Running {
    pub start: DateTime<Local>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CheckRunInfo {
    pub end: DateTime<Local>,
    pub outcome: CheckOutcome,
}

impl CheckRunInfo {
    pub fn new_success() -> Self {
        Self {
            end: Local::now(),
            outcome: CheckOutcome::Success,
        }
    }

    pub fn new_aborted() -> Self {
        Self {
            end: Local::now(),
            outcome: CheckOutcome::Aborted,
        }
    }

    pub fn new_repair(log_collection: LogCollection) -> Self {
        Self {
            end: Local::now(),
            outcome: CheckOutcome::Repair(log_collection),
        }
    }

    pub fn new_error(log_collection: LogCollection) -> Self {
        Self {
            end: Local::now(),
            outcome: CheckOutcome::Error(log_collection),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum CheckOutcome {
    Success,
    Aborted,
    Repair(LogCollection),
    Error(LogCollection),
}
