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

pub enum BackupPrefix {
    Taken,
    OtherEmptyExists,
    EmptyButOtherExists,
}

impl std::fmt::Display for BackupPrefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Self::Taken => write!(
                f,
                "{}",
                gettext("This archive prefix is already used by another backup configuration for the same backup repository.")
            ),
            Self::OtherEmptyExists =>write!(
                f,
                "{} {}",
                gettext("A configuration for the same backup repository with an empty archive prefix exists."),
                gettext("There can only be one configuration for a backup repository if the archive prefix is empty."),
            ),
            Self::EmptyButOtherExists => write!(
                f,
                "{} {}",
                gettext("The archive prefix is empty and a configuration for the same backup repository already exists."),
                gettext("There can only be one configuration for a backup repository if the archive prefix is empty."),
            ),
        }
    }
}
