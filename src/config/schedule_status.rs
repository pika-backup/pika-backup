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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub last_activity: chrono::DateTime<chrono::Local>,
    pub minutes_past: u64,
    /// Will be added to `minutes_past` if the date of `last_activity` is not today.
    pub minutes_today: u64,
}

impl Activity {
    pub fn tick(&mut self) -> bool {
        if self.last_activity.date() < chrono::Local::today() {
            self.last_activity = chrono::Local::now();
            self.minutes_past += self.minutes_today;
            self.minutes_today = 1;

            true
        } else {
            // TODO change to var
            if self.minutes_today < 20 {
                self.minutes_today += 1;
                true
            } else {
                false
            }
        }
    }
}
