pub mod error;

use crate::borg;
use crate::globals::*;

use gio::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path;
use zeroize::Zeroizing;

/// Compatibility config version
pub static VERSION: u16 = 1;

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Ord, Eq, PartialOrd, PartialEq)]
pub struct ConfigId(String);

impl ConfigId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl std::fmt::Display for ConfigId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Backup {
    #[serde(default)]
    pub config_version: u16,
    pub id: ConfigId,
    #[serde(default = "fake_repo_id")]
    pub repo_id: borg::RepoId,
    pub repo: BackupRepo,
    pub encrypted: bool,
    #[serde(default)]
    pub encryption_mode: String,
    pub include: BTreeSet<path::PathBuf>,
    pub exclude: BTreeSet<Pattern>,
}

fn fake_repo_id() -> borg::RepoId {
    borg::RepoId::new(format!(
        "-randomid-{}",
        glib::uuid_string_random().to_string()
    ))
}

impl Backup {
    pub fn new(repo: BackupRepo, info: borg::List, encrypted: bool) -> Self {
        let mut include = std::collections::BTreeSet::new();
        include.insert("".into());
        let mut exclude = std::collections::BTreeSet::new();
        exclude.insert(Pattern::PathPrefix(".cache".into()));

        Self {
            config_version: VERSION,
            id: ConfigId::new(glib::uuid_string_random().to_string()),
            repo,
            repo_id: info.repository.id,
            encrypted,
            encryption_mode: info.encryption.mode,
            include,
            exclude,
        }
    }

    pub fn include_dirs(&self) -> Vec<path::PathBuf> {
        let mut dirs = Vec::new();

        for dir in &self.include {
            dirs.push(absolute(dir));
        }

        dirs
    }

    pub fn exclude_dirs_internal(&self) -> Vec<Pattern> {
        let mut dirs = Vec::new();

        for Pattern::PathPrefix(dir) in &self.exclude {
            dirs.push(Pattern::PathPrefix(absolute(dir)));
        }

        dirs.push(Pattern::PathPrefix(absolute(path::Path::new(
            crate::REPO_MOUNT_DIR,
        ))));

        dirs
    }

    pub fn update_version_0(&mut self, info: borg::List, icon_symbolic_new: Option<gio::Icon>) {
        if self.config_version == 0 {
            self.config_version = 1;

            if let BackupRepo::Local(RepoLocal {
                ref mut icon_symbolic,
                ..
            }) = self.repo
            {
                *icon_symbolic = icon_symbolic_new
                    .and_then(|icon| gio::IconExt::to_string(&icon))
                    .as_ref()
                    .map(ToString::to_string);
            }
            self.repo_id = info.repository.id;
            self.encryption_mode = info.encryption.mode;
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum Pattern {
    PathPrefix(path::PathBuf),
    //Fnmatch(path::PathBuf),
}

impl Pattern {
    pub fn selector(&self) -> String {
        match self {
            Self::PathPrefix(_) => "pp",
            //Self::Fnmatch(_) => "fm",
        }
        .to_string()
    }

    pub fn pattern(&self) -> String {
        match self {
            Self::PathPrefix(p) => p,
            //Self::Fnmatch(p) => p,
        }
        .to_string_lossy()
        .to_string()
    }
}

pub type Password = Zeroizing<Vec<u8>>;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum BackupRepo {
    Local(RepoLocal),
    Remote(RepoRemote),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RepoLocal {
    /// If not absulte, this path is prefixed with the `mount_path`
    path: path::PathBuf,
    #[serde(default = "default_mount_path")]
    mount_path: path::PathBuf,
    pub uri: Option<String>,
    #[serde(alias = "device")]
    pub drive_name: Option<String>,
    #[serde(alias = "label")]
    pub mount_name: Option<String>,
    pub volume_uuid: Option<String>,
    pub removable: bool,
    pub icon: Option<String>,
    pub icon_symbolic: Option<String>,
    pub settings: Option<BackupSettings>,
}

fn default_mount_path() -> path::PathBuf {
    "/".into()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RepoRemote {
    pub uri: String,
    pub settings: Option<BackupSettings>,
}

impl RepoLocal {
    pub fn path(&self) -> std::path::PathBuf {
        self.mount_path.join(&self.path)
    }
}

impl BackupRepo {
    pub fn new_remote(uri: String) -> Self {
        BackupRepo::Remote(RepoRemote {
            uri,
            settings: None,
        })
    }

    pub fn new_local_from_path(path: std::path::PathBuf) -> Self {
        let file = gio::File::new_for_path(&path);

        if let Ok(mount) = file.find_enclosing_mount(Some(&gio::Cancellable::new())) {
            Self::new_local_from_mount(mount, path, file.get_uri().to_string())
        } else {
            BackupRepo::Local(RepoLocal {
                path,
                mount_path: default_mount_path(),
                uri: None,
                icon: None,
                icon_symbolic: None,
                mount_name: None,
                drive_name: None,
                removable: false,
                volume_uuid: None,
                settings: None,
            })
        }
    }

    pub fn new_local_from_mount(
        mount: gio::Mount,
        mut path: std::path::PathBuf,
        uri: String,
    ) -> Self {
        let mut mount_path = "/".into();

        if let Some(mount_root) = mount.get_root().unwrap().get_path() {
            if let Ok(repo_path) = path.strip_prefix(&mount_root) {
                mount_path = mount_root;
                path = repo_path.to_path_buf();
            }
        }

        BackupRepo::Local(RepoLocal {
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
            volume_uuid: get_mount_uuid(&mount),
            settings: None,
        })
    }

    pub fn icon(&self) -> String {
        match self {
            Self::Local(local) => local.icon.clone().unwrap_or_else(|| String::from("folder")),
            Self::Remote(_) => String::from("network-server"),
        }
    }

    pub fn icon_symbolic(&self) -> String {
        match self {
            Self::Local(local) => local
                .icon_symbolic
                .clone()
                .unwrap_or_else(|| String::from("folder-symbolic")),
            Self::Remote(_) => String::from("network-server-symbolic"),
        }
    }

    pub fn location(&self) -> String {
        if let Self::Local(local) = self {
            format!(
                "{} – {}",
                local.mount_name.as_deref().unwrap_or_default(),
                self.get_subtitle(),
            )
        } else {
            self.to_string()
        }
    }

    pub fn get_uri_fuse(&self) -> Option<String> {
        match self {
            Self::Local(RepoLocal { uri: Some(uri), .. })
                if !gio::File::new_for_uri(&uri).is_native() =>
            {
                Some(uri.clone())
            }
            _ => None,
        }
    }

    pub fn get_subtitle(&self) -> String {
        match self {
            Self::Local(local) => local
                .drive_name
                .clone()
                .or_else(|| self.get_uri_fuse())
                .unwrap_or_else(|| self.to_string()),
            Self::Remote(_) => self.to_string(),
        }
    }

    pub fn set_settings(&mut self, settings: Option<BackupSettings>) {
        *match self {
            Self::Local(local) => &mut local.settings,
            Self::Remote(remote) => &mut remote.settings,
        } = settings;
    }

    pub fn get_settings(&self) -> Option<BackupSettings> {
        match self {
            Self::Local(local) => &local.settings,
            Self::Remote(remote) => &remote.settings,
        }
        .clone()
    }
}

impl std::fmt::Display for BackupRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let repo = match self {
            Self::Local(local) => local.path().to_string_lossy().to_string(),
            Self::Remote(remote) => remote.uri.to_string(),
        };
        write!(f, "{}", repo)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BackupSettings {
    pub command_line_args: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Backups(Vec<Backup>);

impl Backups {
    pub fn from_default_path() -> std::io::Result<Self> {
        Self::from_path(&Self::default_path()?)
    }

    pub fn from_path(path: &std::path::Path) -> std::io::Result<Self> {
        #[derive(Serialize, Deserialize, Debug)]
        struct BackupsLegacy {
            backups: BTreeMap<ConfigId, Backup>,
        }

        let conf: std::result::Result<Self, _> =
            serde_json::de::from_reader(std::fs::File::open(&path)?);

        // pre v2 parser
        match conf {
            Ok(conf) => Ok(conf),
            Err(err) => {
                let conf_legacy: std::result::Result<BackupsLegacy, _> =
                    serde_json::de::from_reader(std::fs::File::open(path)?);
                match conf_legacy {
                    Ok(legacy) => Ok(Self(legacy.backups.into_iter().map(|x| x.1).collect())),
                    Err(_) => Err(err.into()),
                }
            }
        }
    }

    pub fn exists(&self, id: &ConfigId) -> bool {
        self.iter().any(|x| x.id == *id)
    }

    pub fn insert(&mut self, new: Backup) -> Result<(), error::BackupExists> {
        if self.exists(&new.id) {
            Err(error::BackupExists { id: new.id })
        } else {
            self.0.push(new);
            Ok(())
        }
    }

    pub fn remove(&mut self, remove: &ConfigId) -> Result<(), error::BackupNotFound> {
        if !self.exists(remove) {
            Err(error::BackupNotFound::new(remove.clone()))
        } else {
            self.0.retain(|x| x.id != *remove);
            Ok(())
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Backup> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Backup> {
        self.0.iter_mut()
    }

    pub fn default_path() -> std::io::Result<std::path::PathBuf> {
        crate::utils::prepare_config_file("config.json", Self::default())
    }
}

pub fn absolute(path: &path::Path) -> path::PathBuf {
    HOME_DIR.join(path)
}

pub fn get_mount_uuid(mount: &gio::Mount) -> Option<String> {
    let volume = mount.get_volume();

    volume
        .as_ref()
        .and_then(gio::Volume::get_uuid)
        .or_else(|| volume.as_ref().and_then(|v| v.get_identifier("uuid")))
        .as_ref()
        .map(std::string::ToString::to_string)
}
