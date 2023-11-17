use crate::config;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ScheduleStatus {
    #[serde(default)]
    pub config_version: super::Version,
    pub activity: BTreeMap<config::ConfigId, Activity>,
}

impl super::ConfigType for ScheduleStatus {
    fn path() -> std::path::PathBuf {
        let mut path = glib::user_config_dir();
        path.push(env!("CARGO_PKG_NAME"));
        path.push("schedule_status.json");

        path
    }
}

impl super::ConfigVersion for ScheduleStatus {
    fn extract_version(json: &serde_json::Value) -> u64 {
        json.as_object()
            .and_then(|d| d.get("config_version"))
            .and_then(|v| v.as_u64())
            .unwrap_or(2)
    }
}

impl crate::utils::LookupConfigId for ScheduleStatus {
    type Item = Activity;

    fn try_get_mut(
        &mut self,
        key: &config::ConfigId,
    ) -> Result<&mut Activity, config::error::BackupNotFound> {
        self.activity.try_get_mut(key)
    }

    fn try_get(&self, key: &config::ConfigId) -> Result<&Activity, config::error::BackupNotFound> {
        self.activity.try_get(key)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Activity {
    pub used: std::time::Duration,
    pub last_update: chrono::DateTime<chrono::Local>,
}

impl Activity {
    pub fn tick(&mut self, length: std::time::Duration) {
        if self.used < crate::schedule::USED_THRESHOLD {
            self.used += length;
            self.last_update = chrono::Local::now();
        }
    }

    pub fn reset(&mut self) {
        self.used = std::time::Duration::ZERO;
        self.last_update = chrono::Local::now();
    }
}

impl Default for Activity {
    fn default() -> Self {
        Self {
            used: Default::default(),
            last_update: chrono::Local::now(),
        }
    }
}
