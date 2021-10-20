use glib::prelude::*;

use crate::borg::RepoId;
use crate::config::ConfigId;

pub type BackupStatus = std::collections::HashMap<RepoId, ConfigId>;

pub fn backup_start() -> gio::SimpleAction {
    gio::SimpleAction::new("backup.start", Some(&String::static_variant_type()))
}

pub fn backup_show() -> gio::SimpleAction {
    gio::SimpleAction::new("backup.show", Some(&String::static_variant_type()))
}
