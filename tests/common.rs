#![allow(dead_code)]

pub use pika_backup::borg::CommandRun;
pub use pika_backup::{borg, config, config::ConfigId};

pub use std::collections::BTreeSet;
pub use std::ffi::OsStr;
pub use std::path::{Path, PathBuf};

pub use tempfile::tempdir;

pub fn config(path: &std::path::Path) -> config::Backup {
    let uuid = glib::uuid_string_random().to_string();
    config::Backup {
        config_version: 1,
        id: ConfigId::new(uuid),
        repo_id: borg::RepoId::new("repo id".into()),
        archive_prefix: config::ArchivePrefix::generate(),
        encryption_mode: "none".into(),
        repo: config::local::Repository::from_path(path.to_path_buf()).into_config(),
        encrypted: false,
        include: Default::default(),
        exclude: Default::default(),
        schedule: Default::default(),
        prune: Default::default(),
    }
}

pub fn tmpdir_config() -> (config::Backup, tempfile::TempDir) {
    let repo_dir = tempdir().unwrap();

    let mut config = config(std::path::Path::new(repo_dir.path()));
    config.include.insert("/dev/null".into());

    (config, repo_dir)
}

pub struct Excluded<'a> {
    pub base_dir: &'a tempfile::TempDir,
    pub paths: Vec<PathBuf>,
}

impl<'a> Excluded<'a> {
    pub fn new(base_dir: &'a tempfile::TempDir) -> Self {
        Self {
            base_dir,
            paths: Vec::new(),
        }
    }

    pub fn add(&mut self, path: impl AsRef<Path>) {
        self.paths.push(path.as_ref().join("exclude"));
        touch(self.base_dir, &path.as_ref().join("exclude"));
    }

    pub fn test(&self, exclude_rules: &BTreeSet<config::Exclude>) {
        for path in &self.paths {
            let full_path = self.base_dir.path().join(path);
            assert!(
                exclude_rules.iter().any(|rule| rule.is_match(&full_path)),
                "Should be excluded: {:?}",
                path
            );
        }
    }
}

pub struct Included<'a> {
    pub base_dir: &'a tempfile::TempDir,
    pub paths: Vec<PathBuf>,
}

impl<'a> Included<'a> {
    pub fn new(base_dir: &'a tempfile::TempDir) -> Self {
        Self {
            base_dir,
            paths: Vec::new(),
        }
    }

    pub fn add(&mut self, path: impl AsRef<Path>) {
        self.paths.push(path.as_ref().to_path_buf());
        touch(self.base_dir, &path.as_ref().join("include"));
    }

    pub fn test(&self, exclude_rules: &BTreeSet<config::Exclude>) {
        for path in &self.paths {
            let full_path = self.base_dir.path().join(path);
            assert!(
                exclude_rules.iter().all(|rule| !rule.is_match(&full_path)),
                "Should not be excluded: {:?}",
                path
            );
        }
    }
}

pub fn touch(dir: &tempfile::TempDir, path: &Path) {
    let full_path = dir.path().join(path);
    std::fs::create_dir_all(full_path.parent().unwrap()).unwrap();
    std::fs::File::create(full_path).unwrap();
}
