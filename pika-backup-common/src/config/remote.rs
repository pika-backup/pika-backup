use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Repository {
    pub uri: String,
    pub settings: Option<super::BackupSettings>,
}

impl Repository {
    pub const fn from_uri(uri: String) -> Self {
        Self {
            uri,
            settings: None,
        }
    }

    pub const fn into_config(self) -> super::Repository {
        super::Repository::Remote(self)
    }
}
