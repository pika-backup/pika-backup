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
    /// Outer volume if not equal to `volume_uuid_identifier`
    ///
    ///  Retrieved via `gio::Volume::uuid()`
    pub volume_uuid: Option<String>,
    /// Inner volume if not equal to `volume_uuid`
    ///
    /// Retrieved via `gio::Volume::identifier("uuid")`
    pub volume_uuid_identifier: Option<String>,
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

        match file.find_enclosing_mount(Some(&gio::Cancellable::new())) {
            Ok(mount) => Self::from_mount(mount, path, file.uri().to_string()),
            _ => {
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
                    volume_uuid_identifier: None,
                    settings: None,
                }
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
            removable: mount.drive().as_ref().is_some_and(gio::Drive::is_removable),
            volume_uuid: mount.volume().and_then(|v| v.uuid()).map(|x| x.to_string()),
            volume_uuid_identifier: mount
                .volume()
                .and_then(|v| v.identifier("uuid"))
                .map(|x| x.to_string()),
            settings: None,
        }
    }

    pub fn path(&self) -> std::path::PathBuf {
        self.mount_path.join(&self.path)
    }

    pub fn is_likely_on_volume(&self, volume: &gio::Volume) -> bool {
        let new_path = volume
            .get_mount()
            .and_then(|x| x.root().path())
            .and_then(|x| x.canonicalize().ok());

        if new_path.is_some() && self.mount_path.canonicalize().ok() == new_path {
            return true;
        }

        let new_uuids = [volume.uuid(), volume.identifier("uuid")]
            .into_iter()
            .flatten()
            .map(|x| x.to_string())
            .collect::<Vec<_>>();
        if [
            self.volume_uuid.as_ref(),
            self.volume_uuid_identifier.as_ref(),
        ]
        .iter()
        .flatten()
        .any(|&x| new_uuids.contains(&x.into()))
        {
            return true;
        }

        false
    }

    pub const fn into_config(self) -> super::Repository {
        super::Repository::Local(self)
    }
}
