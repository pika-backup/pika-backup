use crate::borg;
use crate::prelude::*;

use gio::prelude::*;
use std::collections::BTreeSet;
use std::path;

use super::{absolute, error, local, ConfigType, Pattern, Prune, Repository, Schedule};

/// Compatibility config version
pub const VERSION: u16 = 2;

#[derive(
    Serialize, Deserialize, Clone, Debug, Hash, Ord, Eq, PartialOrd, PartialEq, zbus::zvariant::Type,
)]
pub struct ConfigId(String);

impl ConfigId {
    pub const fn new(id: String) -> Self {
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

impl glib::ToVariant for ConfigId {
    fn to_variant(&self) -> glib::Variant {
        self.as_str().to_variant()
    }
}

impl glib::FromVariant for ConfigId {
    fn from_variant(variant: &glib::Variant) -> Option<Self> {
        let id = glib::FromVariant::from_variant(variant)?;
        Some(Self::new(id))
    }
}

impl glib::StaticVariantType for ConfigId {
    fn static_variant_type() -> std::borrow::Cow<'static, glib::VariantTy> {
        String::static_variant_type()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Backup {
    #[serde(default)]
    pub config_version: u16,
    pub id: ConfigId,
    #[serde(default)]
    pub archive_prefix: ArchivePrefix,
    #[serde(default = "fake_repo_id")]
    pub repo_id: borg::RepoId,
    pub repo: Repository,
    pub encrypted: bool,
    #[serde(default)]
    pub encryption_mode: String,
    pub include: BTreeSet<path::PathBuf>,
    pub exclude: BTreeSet<Pattern>,
    #[serde(default)]
    pub schedule: Schedule,
    #[serde(default)]
    pub prune: Prune,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ArchivePrefix(pub String);

impl ArchivePrefix {
    pub fn new(prefix: &str) -> Self {
        Self(prefix.trim().to_string())
    }

    pub fn generate() -> Self {
        Self(format!(
            "{}-",
            glib::uuid_string_random()
                .chars()
                .take(6)
                .collect::<String>()
        ))
    }
}

impl Default for ArchivePrefix {
    fn default() -> Self {
        Self::generate()
    }
}

impl std::fmt::Display for ArchivePrefix {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn fake_repo_id() -> borg::RepoId {
    borg::RepoId::new(format!("-randomid-{}", glib::uuid_string_random()))
}

impl Backup {
    pub fn new(repo: Repository, info: borg::List, encrypted: bool) -> Self {
        let mut include = std::collections::BTreeSet::new();
        include.insert("".into());
        let mut exclude = std::collections::BTreeSet::new();
        exclude.insert(Pattern::cache());
        exclude.insert(Pattern::flatpak_app_cache());

        Self {
            config_version: VERSION,
            id: ConfigId::new(glib::uuid_string_random().to_string()),
            archive_prefix: ArchivePrefix::generate(),
            repo,
            repo_id: info.repository.id,
            encrypted,
            encryption_mode: info.encryption.mode,
            include,
            exclude,
            schedule: Default::default(),
            prune: Default::default(),
        }
    }

    #[cfg(test)]
    pub fn test_new_mock() -> Backup {
        let info = borg::List {
            archives: vec![],
            encryption: borg::Encryption {
                mode: String::from("none"),
                keyfile: None,
            },
            repository: borg::Repository {
                id: fake_repo_id(),
                last_modified: chrono::MIN_DATETIME.naive_utc(),
                location: std::path::PathBuf::new(),
            },
        };
        let repo =
            local::Repository::from_path(std::path::PathBuf::from("/tmp/INVALID")).into_config();
        Backup::new(repo, info, false)
    }

    pub fn include_dirs(&self) -> BTreeSet<path::PathBuf> {
        let mut dirs = BTreeSet::new();

        for dir in &self.include {
            dirs.insert(absolute(dir));
        }

        dirs
    }

    pub fn exclude_dirs_internal(&self) -> BTreeSet<Pattern> {
        let mut dirs = BTreeSet::new();

        for pattern in &self.exclude {
            match pattern {
                Pattern::PathPrefix(dir) => dirs.insert(Pattern::PathPrefix(absolute(dir))),
                other => dirs.insert(other.clone()),
            };
        }

        dirs.insert(Pattern::PathPrefix(absolute(path::Path::new(
            crate::REPO_MOUNT_DIR,
        ))));

        dirs
    }

    pub fn update_version_0(&mut self, info: borg::List, icon_symbolic_new: Option<gio::Icon>) {
        if self.config_version == 0 {
            self.config_version = 1;

            if let Repository::Local(local::Repository {
                ref mut icon_symbolic,
                ..
            }) = self.repo
            {
                *icon_symbolic = icon_symbolic_new
                    .and_then(|icon| IconExt::to_string(&icon))
                    .as_ref()
                    .map(ToString::to_string);
            }
            self.repo_id = info.repository.id;
            self.encryption_mode = info.encryption.mode;
        }
    }
}

impl LookupConfigId for Backups {
    type Item = Backup;
    fn get_result_mut(&mut self, key: &ConfigId) -> Result<&mut Backup, error::BackupNotFound> {
        self.iter_mut()
            .find(|x| x.id == *key)
            .ok_or_else(|| error::BackupNotFound::new(key.clone()))
    }

    fn get_result(&self, key: &ConfigId) -> Result<&Backup, error::BackupNotFound> {
        self.iter()
            .find(|x| x.id == *key)
            .ok_or_else(|| error::BackupNotFound::new(key.clone()))
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct Backups(Vec<Backup>);

impl ConfigType for Backups {
    fn path() -> std::path::PathBuf {
        let mut path = glib::user_config_dir();
        path.push(env!("CARGO_PKG_NAME"));
        path.push("backup.json");

        path
    }
}

impl Backups {
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
}
