use gio::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
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
        let file = gio::File::for_path(&path);

        if let Ok(mount) = file.find_enclosing_mount(Some(&gio::Cancellable::new())) {
            Self::from_mount(mount, path, file.uri().to_string())
        } else {
            let mount_entry = gio::UnixMountEntry::for_file_path(&path).0;

            Self {
                path,
                mount_path: default_mount_path(),
                uri: None,
                icon: mount_entry
                    .as_ref()
                    .and_then(|x| IconExt::to_string(&x.guess_icon()).map(|x| x.to_string())),
                icon_symbolic: mount_entry.as_ref().and_then(|x| {
                    IconExt::to_string(&x.guess_symbolic_icon()).map(|x| x.to_string())
                }),
                mount_name: mount_entry.map(|x| x.guess_name().to_string()),
                drive_name: None,
                removable: false,
                volume_uuid: None,
                settings: None,
            }
        }
    }

    pub fn from_mount(mount: gio::Mount, mut path: std::path::PathBuf, uri: String) -> Self {
        let mut mount_path = "/".into();

        if let Some(mount_root) = mount.root().path() {
            if let Ok(repo_path) = path.strip_prefix(&mount_root) {
                mount_path = mount_root;
                path = repo_path.to_path_buf();
            }
        }

        Self {
            path,
            mount_path,
            uri: Some(uri),
            icon: IconExt::to_string(&mount.icon()).map(Into::into),
            icon_symbolic: IconExt::to_string(&mount.symbolic_icon()).map(Into::into),
            mount_name: Some(mount.name().to_string()),
            drive_name: mount.drive().as_ref().map(gio::Drive::name).map(Into::into),
            removable: mount
                .drive()
                .as_ref()
                .map_or(false, gio::Drive::is_removable),
            volume_uuid: crate::utils::mount_uuid(&mount),
            settings: None,
        }
    }

    pub fn path(&self) -> std::path::PathBuf {
        self.mount_path.join(&self.path)
    }

    pub const fn into_config(self) -> super::Repository {
        super::Repository::Local(self)
    }
}
