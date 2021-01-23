use std::io::prelude::*;

use crate::borg;
use crate::globals::*;

use chrono::prelude::*;
use gio::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path;
use zeroize::Zeroizing;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BackupConfig {
    #[serde(default)]
    pub config_version: u16,
    pub id: String,
    #[serde(default = "fake_repo_id")]
    pub repo_id: String,
    pub repo: BackupRepo,
    pub encrypted: bool,
    #[serde(default)]
    pub encryption_mode: String,
    pub include: BTreeSet<path::PathBuf>,
    pub exclude: BTreeSet<Pattern>,
    pub last_run: Option<RunInfo>,
}

fn fake_repo_id() -> String {
    format!("-randomid-{}", glib::uuid_string_random().to_string())
}

impl BackupConfig {
    pub fn new(repo: BackupRepo, info: borg::List, encrypted: bool) -> Self {
        let mut include = std::collections::BTreeSet::new();
        include.insert("".into());
        let mut exclude = std::collections::BTreeSet::new();
        exclude.insert(Pattern::PathPrefix(".cache".into()));

        Self {
            config_version: crate::CONFIG_VERSION,
            id: glib::uuid_string_random().to_string(),
            repo,
            repo_id: info.repository.id,
            encrypted,
            encryption_mode: info.encryption.mode,
            include,
            exclude,
            last_run: None,
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

            if let BackupRepo::Local {
                ref mut icon_symbolic,
                ..
            } = self.repo
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RunInfo {
    pub end: DateTime<Local>,
    pub result: Result<borg::Stats, String>,
}

impl RunInfo {
    pub fn new(result: Result<borg::Stats, String>) -> Self {
        Self {
            end: Local::now(),
            result,
        }
    }
}

pub type Password = Zeroizing<Vec<u8>>;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum BackupRepo {
    Local {
        path: path::PathBuf,
        uri: Option<String>,
        #[serde(alias = "device")]
        drive_name: Option<String>,
        #[serde(alias = "label")]
        mount_name: Option<String>,
        volume_uuid: Option<String>,
        removable: bool,
        icon: Option<String>,
        icon_symbolic: Option<String>,
        settings: Option<BackupSettings>,
    },
    Remote {
        uri: String,
        settings: Option<BackupSettings>,
    },
}

impl BackupRepo {
    pub fn new_remote(uri: String) -> Self {
        BackupRepo::Remote {
            uri,
            settings: None,
        }
    }

    pub fn new_local_for_file(file: &gio::File) -> Option<Self> {
        if let (Some(path), Some(mount)) = (
            file.get_path(),
            file.find_enclosing_mount(Some(&gio::Cancellable::new()))
                .ok(),
        ) {
            Some(Self::new_local_from_mount(
                mount,
                path,
                file.get_uri().to_string(),
            ))
        } else if let Some(path) = file.get_path() {
            Some(Self::new_local_from_path(path))
        } else {
            None
        }
    }

    pub fn new_local_from_path(path: std::path::PathBuf) -> Self {
        BackupRepo::Local {
            path,
            uri: None,
            icon: None,
            icon_symbolic: None,
            mount_name: None,
            drive_name: None,
            removable: false,
            volume_uuid: None,
            settings: None,
        }
    }

    pub fn new_local_from_mount(mount: gio::Mount, path: std::path::PathBuf, uri: String) -> Self {
        BackupRepo::Local {
            path,
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
        }
    }

    pub fn icon(&self) -> String {
        match self {
            Self::Local { icon, .. } => icon.clone().unwrap_or_else(|| String::from("folder")),
            Self::Remote { .. } => String::from("network-server"),
        }
    }

    pub fn icon_symbolic(&self) -> String {
        match self {
            Self::Local { icon_symbolic, .. } => icon_symbolic
                .clone()
                .unwrap_or_else(|| String::from("folder-symbolic")),
            Self::Remote { .. } => String::from("network-server-symbolic"),
        }
    }

    pub fn get_uri_fuse(&self) -> Option<String> {
        match self {
            Self::Local { uri: Some(uri), .. } if !gio::File::new_for_uri(&uri).is_native() => {
                Some(uri.clone())
            }
            _ => None,
        }
    }

    pub fn get_subtitle(&self) -> String {
        match self {
            Self::Local { ref drive_name, .. } => drive_name
                .clone()
                .or_else(|| self.get_uri_fuse())
                .unwrap_or_else(|| self.to_string()),
            Self::Remote { .. } => self.to_string(),
        }
    }

    pub fn set_settings(&mut self, settings: Option<BackupSettings>) {
        *match self {
            Self::Local {
                ref mut settings, ..
            } => settings,
            Self::Remote {
                ref mut settings, ..
            } => settings,
        } = settings;
    }

    pub fn get_settings(&self) -> Option<BackupSettings> {
        match self {
            Self::Local { settings, .. } => settings,
            Self::Remote { settings, .. } => settings,
        }
        .clone()
    }
}

impl std::fmt::Display for BackupRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let repo = match self {
            Self::Local { path, .. } => path.to_string_lossy().to_string(),
            Self::Remote { uri, .. } => uri.to_string(),
        };
        write!(f, "{}", repo)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BackupSettings {
    pub command_line_args: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct Settings {
    pub backups: BTreeMap<String, BackupConfig>,
}

impl Settings {
    pub fn from_path(path: &std::path::Path) -> std::io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let conf: Self = serde_json::de::from_reader(file)?;
        Ok(conf)
    }

    pub fn default_path() -> std::io::Result<std::path::PathBuf> {
        let mut path = crate::globals::CONFIG_DIR.clone();
        path.push(env!("CARGO_PKG_NAME"));
        std::fs::create_dir_all(&path)?;
        path.push("config.json");

        if let Ok(mut file) = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            file.write_all(b"{ }")?;
        }

        Ok(path)
    }
}

pub fn absolute(path: &path::Path) -> path::PathBuf {
    HOME_DIR.join(path)
}

fn get_mount_uuid(mount: &gio::Mount) -> Option<String> {
    let volume = mount.get_volume();

    volume
        .as_ref()
        .and_then(gio::Volume::get_uuid)
        .or_else(|| volume.as_ref().and_then(|v| v.get_identifier("uuid")))
        .as_ref()
        .map(std::string::ToString::to_string)
}
