mod backup;
pub mod error;
pub mod history;
mod loadable;
pub mod local;
mod pattern;
mod prune;
pub mod remote;
mod repository;
mod schedule;
mod schedule_status;
mod writeable;

pub use backup::*;
pub use history::Histories;
pub use loadable::{ConfigType, Loadable, TrackChanges};
pub use pattern::*;
pub use prune::*;
pub use repository::*;
pub use schedule::*;
pub use schedule_status::*;
pub use writeable::{ArcSwapWriteable, Writeable};

use crate::prelude::*;

use std::collections::HashMap;
use std::path;
use zeroize::Zeroizing;

#[derive(Clone, Default)]
pub struct Password(Zeroizing<String>);

impl Password {
    pub fn new(password: String) -> Self {
        Self(Zeroizing::new(password))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    pub fn libsecret_schema() -> libsecret::Schema {
        libsecret::Schema::new(
            &crate::app_id(),
            libsecret::SchemaFlags::NONE,
            HashMap::from([("repo-id", libsecret::SchemaAttributeType::String)]),
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BackupSettings {
    pub command_line_args: Option<Vec<String>>,
}

pub fn display_path(path: &path::Path) -> String {
    if path.iter().next().is_none() {
        gettext("Home")
    } else {
        path.display().to_string()
    }
}

pub fn absolute(path: &path::Path) -> path::PathBuf {
    glib::home_dir().join(path)
}
