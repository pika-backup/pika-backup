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
