use crate::config;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScheduleStatus {
    pub activity: BTreeMap<config::ConfigId, Activity>,
}

impl ScheduleStatus {
    pub fn default_path() -> std::io::Result<std::path::PathBuf> {
        crate::utils::prepare_config_file("schedule_status.json", Self::default())
    }

    pub fn load() -> std::io::Result<Self> {
        Ok(serde_json::de::from_reader(std::fs::File::open(
            Self::default_path()?,
        )?)?)
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub used: std::time::Duration,
}

impl Activity {
    pub fn tick(&mut self) {
        if self.used < crate::daemon::schedule::USED_THRESHOLD {
            self.used += std::time::Duration::from_secs(60);
        }
    }
}
