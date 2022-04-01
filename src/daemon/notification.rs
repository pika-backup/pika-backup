use crate::config;

pub enum Note<'a> {
    Postponed(&'a config::ConfigId),
    DeviceRequired(&'a config::ConfigId),
    DeviceAvailable(&'a str),
}

impl<'a> Note<'a> {
    pub fn to_string(&self) -> String {
        match self {
            Self::Postponed(id) => format!("postponed-{}", id),
            Self::DeviceRequired(id) => format!("device-required-{}", id),
            Self::DeviceAvailable(id) => format!("device-available-{}", id),
        }
    }
}
