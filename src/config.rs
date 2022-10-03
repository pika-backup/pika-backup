mod backup;
pub mod error;
pub mod exclude;
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
pub use exclude::Exclude;
pub use history::Histories;
pub use loadable::{ConfigType, Loadable, TrackChanges};
pub use pattern::*;
pub use prune::*;
pub use repository::*;
pub use schedule::*;
pub use schedule_status::*;
pub use writeable::{ArcSwapWriteable, Writeable};

use crate::prelude::*;

use std::path;
use zeroize::Zeroizing;

#[derive(Clone, Default)]
pub struct Password(Zeroizing<Vec<u8>>);

impl Password {
    pub fn new(password: String) -> Self {
        Self(Zeroizing::new(password.into_bytes()))
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl From<Zeroizing<Vec<u8>>> for Password {
    fn from(password: Zeroizing<Vec<u8>>) -> Self {
        Self(password)
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

pub fn path_absolute(path: impl AsRef<path::Path>) -> path::PathBuf {
    if path.as_ref().starts_with("/") {
        path.as_ref().to_path_buf()
    } else {
        glib::home_dir().join(path)
    }
}

pub fn path_relative(path: impl AsRef<path::Path>) -> path::PathBuf {
    path.as_ref()
        .strip_prefix(glib::home_dir())
        .map(|x| x.to_path_buf())
        .unwrap_or_else(|_| path.as_ref().to_path_buf())
}
