use gio::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Repository {
    /// If not absulte, this path is prefixed with the `mount_path`
    pub path: std::path::PathBuf,
    #[serde(default = "default_mount_path")]
    pub mount_path: std::path::PathBuf,
    pub uri: Option<String>,
    #[serde(alias = "device")]
    pub drive_name: Option<String>,
    #[serde(alias = "label")]
    pub mount_name: Option<String>,
    pub volume_uuid: Option<String>,
    pub removable: bool,
    pub icon: Option<String>,
    pub icon_symbolic: Option<String>,
    pub settings: Option<super::BackupSettings>,
}

fn default_mount_path() -> std::path::PathBuf {
    "/".into()
}

impl Repository {
    pub fn from_path(path: std::path::PathBuf) -> Self {
        let file = gio::File::new_for_path(&path);

        if let Ok(mount) = file.find_enclosing_mount(Some(&gio::Cancellable::new())) {
            Self::from_mount(mount, path, file.get_uri().to_string())
        } else {
            let mount_entry = gio::UnixMountEntry::new_for(&path).0;

            Self {
                path,
                mount_path: default_mount_path(),
                uri: None,
                icon: gio::IconExt::to_string(&mount_entry.guess_icon().unwrap())
                    .map(|x| x.to_string()),
                icon_symbolic: gio::IconExt::to_string(&mount_entry.guess_symbolic_icon().unwrap())
                    .map(|x| x.to_string()),
                mount_name: Some(mount_entry.guess_name().unwrap().to_string()),
                drive_name: None,
                removable: false,
                volume_uuid: None,
                settings: None,
            }
        }
    }

    pub fn from_mount(mount: gio::Mount, mut path: std::path::PathBuf, uri: String) -> Repository {
        let mut mount_path = "/".into();

        if let Some(mount_root) = mount.get_root().unwrap().get_path() {
            if let Ok(repo_path) = path.strip_prefix(&mount_root) {
                mount_path = mount_root;
                path = repo_path.to_path_buf();
            }
        }

        Self {
            path,
            mount_path,
            uri: Some(uri),
            icon: mount
                .get_icon()
                .as_ref()
                .and_then(gio::IconExt::to_string)
                .map(Into::into),
            icon_symbolic: mount
                .get_symbolic_icon()
                .as_ref()
                .and_then(gio::IconExt::to_string)
                .map(Into::into),
            mount_name: mount.get_name().map(Into::into),
            drive_name: mount
                .get_drive()
                .as_ref()
                .and_then(gio::Drive::get_name)
                .map(Into::into),
            removable: mount
                .get_drive()
                .as_ref()
                .map_or(false, gio::Drive::is_removable),
            volume_uuid: crate::utils::get_mount_uuid(&mount),
            settings: None,
        }
    }

    pub fn path(&self) -> std::path::PathBuf {
        self.mount_path.join(&self.path)
    }

    pub fn into_config(self) -> super::Repository {
        super::Repository::Local(self)
    }
}
