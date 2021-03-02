#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Repository {
    pub uri: String,
    pub settings: Option<super::BackupSettings>,
}

impl Repository {
    pub fn from_uri(uri: String) -> Self {
        Self {
            uri,
            settings: None,
        }
    }

    pub fn into_config(self) -> super::Repository {
        super::Repository::Remote(self)
    }
}
