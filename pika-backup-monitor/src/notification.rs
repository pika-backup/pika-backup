use common::config;

pub enum Note<'a> {
    Postponed(&'a config::ConfigId),
    DeviceRequired(&'a config::ConfigId),
    DeviceAvailable(&'a str),
}

impl std::fmt::Display for Note<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Postponed(id) => write!(f, "postponed-{id}"),
            Self::DeviceRequired(id) => write!(f, "device-required-{id}"),
            Self::DeviceAvailable(id) => write!(f, "device-available-{id}"),
        }
    }
}
