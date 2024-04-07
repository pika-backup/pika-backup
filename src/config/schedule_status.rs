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

/// System activity monitoring
///
/// We increment this at regular intervals while the system is running up to USED_THRESHOLD.
///
/// This is used to determine whether we should start a backup. We only start backups when
/// the system has been "in use" for 10 minutes *or* the accumulated in-use-time is higher
/// than 10 minutes. (to prevent backups from never running if the system is never in use for
/// longer than 10 minutes)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Activity {
    /// The total time
    pub used: std::time::Duration,
    /// The time of the last update
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

    pub fn time_until_threshold(&self) -> std::time::Duration {
        crate::schedule::USED_THRESHOLD.saturating_sub(self.used)
    }

    pub fn is_threshold_reached(&self) -> bool {
        self.time_until_threshold().is_zero()
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
