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

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
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
