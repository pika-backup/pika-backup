use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Schedule {
    pub enabled: bool,
    #[serde(default)]
    pub settings: Settings,
    pub frequency: Frequency,
}

/// User configured settings to the schedule algorithm.
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
pub struct Settings {
    /// Run backups regardless of battery status
    pub run_on_battery: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum Frequency {
    Hourly,
    Daily { preferred_time: chrono::NaiveTime },
    Weekly { preferred_weekday: chrono::Weekday },
    Monthly { preferred_day: u8 },
}

impl Default for Frequency {
    fn default() -> Self {
        Self::Daily {
            preferred_time: chrono::NaiveTime::from_hms_opt(17, 00, 00)
                .expect("17:00 as a naive time must always exist"),
        }
    }
}

impl Frequency {
    pub fn name(&self) -> String {
        match self {
            Self::Hourly => gettext("Hourly"),
            Self::Daily { .. } => gettext("Daily"),
            Self::Weekly { .. } => gettext("Weekly"),
            Self::Monthly { .. } => gettext("Monthly"),
        }
    }
}
