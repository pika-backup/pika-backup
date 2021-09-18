use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Schedule {
    pub enabled: bool,
    pub frequency: Frequency,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Frequency {
    Hourly,
    Daily { preferred_time: chrono::NaiveTime },
    Weekly { preferred_weekday: chrono::Weekday },
    Monthly { preferred_day: u8 },
}

impl Default for Frequency {
    fn default() -> Self {
        Self::Daily {
            preferred_time: chrono::NaiveTime::from_hms(17, 00, 00),
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
