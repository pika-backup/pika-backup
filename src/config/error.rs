use crate::prelude::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BackupExists {
    pub id: super::ConfigId,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct BackupNotFound {
    pub id: super::ConfigId,
}

impl BackupNotFound {
    pub const fn new(id: super::ConfigId) -> Self {
        Self { id }
    }
}

pub struct BackupPrefixTaken;

impl std::fmt::Display for BackupPrefixTaken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", gettext("This archive prefix is already set for another backup configuration using the same repository."))
    }
}
