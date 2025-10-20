use std::collections::{BTreeMap, BTreeSet};
use std::path;

use gio::prelude::*;
use serde::{Deserialize, Serialize};

use super::loadable::ConfigVersion;
use super::{
    ABSOLUTE, ConfigType, Exclude, Pattern, Prune, RELATIVE, Repository, Schedule, absolute, error,
    exclude,
};
use crate::borg;
use crate::prelude::*;

#[derive(
    Serialize,
    Deserialize,
    Clone,
    Debug,
    Hash,
    Ord,
    Eq,
    PartialOrd,
    PartialEq,
    zbus::zvariant::Type,
    glib::ValueDelegate,
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

impl ToVariant for ConfigId {
    fn to_variant(&self) -> glib::Variant {
        self.as_str().to_variant()
    }
}

impl FromVariant for ConfigId {
    fn from_variant(variant: &glib::Variant) -> Option<Self> {
        let id = FromVariant::from_variant(variant)?;
        Some(Self::new(id))
    }
}

impl StaticVariantType for ConfigId {
    fn static_variant_type() -> std::borrow::Cow<'static, glib::VariantTy> {
        String::static_variant_type()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UserScriptKind {
    PreBackup,
    PostBackup,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, glib::Boxed)]
#[boxed_type(name = "PkBackupConfig")]
pub struct Backup {
    #[serde(default)]
    pub config_version: super::Version,
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
    pub exclude: BTreeSet<Exclude<{ RELATIVE }>>,
    #[serde(default)]
    pub schedule: Schedule,
    #[serde(default)]
    pub prune: Prune,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub user_scripts: BTreeMap<UserScriptKind, String>,
}

impl Backup {
    pub fn new(repo: Repository, info: borg::List, encrypted: bool) -> Self {
        let mut include = std::collections::BTreeSet::new();
        include.insert("".into());
        let mut exclude = std::collections::BTreeSet::new();
        exclude.insert(Exclude::Predefined(exclude::Predefined::Caches));

        Self {
            config_version: Default::default(),
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
            title: Default::default(),
            user_scripts: Default::default(),
        }
    }

    pub fn title(&self) -> String {
        if self.title.trim().is_empty() {
            self.repo.title_fallback()
        } else {
            self.title.clone()
        }
    }

    #[cfg(feature = "test")]
    pub fn test_new_mock() -> Backup {
        let info = borg::List {
            archives: vec![],
            encryption: borg::Encryption {
                mode: String::from("none"),
                keyfile: None,
            },
            repository: borg::Repository {
                id: fake_repo_id(),
                last_modified: chrono::DateTime::<chrono::Utc>::MIN_UTC.naive_utc(),
                location: std::path::PathBuf::new(),
            },
        };
        let repo = super::local::Repository::from_path(std::path::PathBuf::from("/tmp/INVALID"))
            .into_config();
        Backup::new(repo, info, false)
    }

    pub fn set_archive_prefix<'a>(
        &mut self,
        prefix: ArchivePrefix,
        configs: impl Iterator<Item = &'a Self> + Clone,
    ) -> Result<(), error::BackupPrefix> {
        self.is_archive_prefix_ok(&prefix, configs)?;

        self.archive_prefix = prefix;
        Ok(())
    }

    pub fn is_archive_prefix_ok<'a>(
        &self,
        prefix: &ArchivePrefix,
        configs: impl Iterator<Item = &'a Self> + Clone,
    ) -> Result<(), error::BackupPrefix> {
        let other_configs = configs.filter(|x| x.repo_id == self.repo_id && x.id != self.id);

        if other_configs.clone().any(|x| &x.archive_prefix == prefix) {
            Err(error::BackupPrefix::Taken)
        } else if other_configs.clone().any(|x| x.archive_prefix.is_empty()) {
            Err(error::BackupPrefix::OtherEmptyExists)
        } else if prefix.is_empty() && other_configs.clone().next().is_some() {
            Err(error::BackupPrefix::EmptyButOtherExists)
        } else {
            Ok(())
        }
    }

    pub fn include_dirs(&self) -> BTreeSet<path::PathBuf> {
        let mut dirs = BTreeSet::new();

        for dir in &self.include {
            dirs.insert(absolute(dir));
        }

        dirs
    }

    pub fn exclude_dirs_internal(&self) -> BTreeSet<Exclude<{ ABSOLUTE }>> {
        let mut dirs =
            BTreeSet::from_iter(self.exclude.clone().into_iter().map(|x| x.into_absolute()));

        if *crate::globals::APP_IS_SANDBOXED {
            dirs.insert(Exclude::from_pattern(Pattern::path_prefix(format!(
                ".var/app/{}/data/flatpak/",
                crate::APP_ID
            ))));
        }

        dirs
    }

    pub fn set_mount_path(&mut self, mount: &gio::Mount) {
        if let Some(new_mount_path) = mount.root().path() {
            match self.repo {
                super::Repository::Local(ref mut repo @ super::local::Repository { .. }) => {
                    if repo.mount_path != new_mount_path {
                        tracing::warn!(
                            "Repository mount path seems to have changed. Trying with this: {:?}",
                            new_mount_path
                        );

                        repo.mount_path = new_mount_path;
                    } else {
                        tracing::debug!("Mount path still the same");
                    }
                }
                _ => unreachable!(),
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash, glib::Boxed)]
#[boxed_type(name = "PkArchivePrefixConfig", nullable)]
pub struct ArchivePrefix(pub String);

/**
```
# use pika_backup_common::config::ArchivePrefix;
assert_eq!(ArchivePrefix::new("x").to_string(), String::from("x-"));
assert_eq!(ArchivePrefix::new(" x-").to_string(), String::from("x-"));
assert_eq!(ArchivePrefix::new("").to_string(), String::from(""));
```
**/
impl ArchivePrefix {
    pub fn new(prefix: &str) -> Self {
        let mut result = prefix.trim().to_string();
        if !matches!(result.chars().last(), Some('-') | None) {
            result.push('-');
        }

        Self(result)
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

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
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

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct Backups(Vec<Backup>);

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

impl LookupConfigId for Backups {
    type Item = Backup;
    fn try_get_mut(&mut self, key: &ConfigId) -> Result<&mut Backup, error::BackupNotFound> {
        self.iter_mut()
            .find(|x| x.id == *key)
            .ok_or_else(|| error::BackupNotFound::new(key.clone()))
    }

    fn try_get(&self, key: &ConfigId) -> Result<&Backup, error::BackupNotFound> {
        self.iter()
            .find(|x| x.id == *key)
            .ok_or_else(|| error::BackupNotFound::new(key.clone()))
    }
}

impl ConfigType for Backups {
    fn path() -> std::path::PathBuf {
        let mut path = glib::user_config_dir();
        path.push("pika-backup");
        path.push("backup.json");

        path
    }
}

impl ConfigVersion for Backups {
    /// Backup configurations < 2 are not supported anymore
    fn version_compatible(version: u64) -> bool {
        (2..=super::VERSION).contains(&version)
    }

    fn extract_version(json: &serde_json::Value) -> u64 {
        json.as_array()
            .and_then(|a| a.first())
            .and_then(|v| v.get("config_version"))
            .and_then(|v| v.as_u64())
            .unwrap_or(2)
    }
}
