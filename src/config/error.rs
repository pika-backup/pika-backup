#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BackupExists {
    pub id: super::ConfigId,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BackupNotFound {
    pub id: super::ConfigId,
}

impl BackupNotFound {
    pub fn new(id: super::ConfigId) -> Self {
        Self { id }
    }
}
