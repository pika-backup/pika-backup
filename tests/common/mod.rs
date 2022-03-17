pub use pika_backup::borg::CommandRun;
use pika_backup::{borg, config, config::ConfigId};
pub use std::path::{Path, PathBuf};

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
