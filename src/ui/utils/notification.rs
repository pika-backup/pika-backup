use crate::config;

pub enum BackupNote<'a> {
    Warnings(&'a config::ConfigId),
    Failed(&'a config::ConfigId),
}

impl std::fmt::Display for BackupNote<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Warnings(id) => write!(f, "backup-warnings-{id}"),
            Self::Failed(id) => write!(f, "backup-failed-{id}"),
        }
    }
}
