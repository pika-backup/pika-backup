use crate::config;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ScheduleStatus {
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

impl crate::utils::LookupConfigId for ScheduleStatus {
    type Item = Activity;

    fn get_result_mut(
        &mut self,
        key: &config::ConfigId,
    ) -> Result<&mut Activity, config::error::BackupNotFound> {
        self.activity.get_result_mut(key)
    }

    fn get_result(
        &self,
        key: &config::ConfigId,
    ) -> Result<&Activity, config::error::BackupNotFound> {
        self.activity.get_result(key)
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Activity {
    pub used: std::time::Duration,
}

impl Activity {
    pub fn tick(&mut self) {
        if self.used < crate::schedule::USED_THRESHOLD {
            self.used += std::time::Duration::from_secs(60);
        }
    }
}
