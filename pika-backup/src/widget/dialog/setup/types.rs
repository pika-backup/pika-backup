use std::str::FromStr;

use common::{borg, config};
use gio::prelude::*;

use crate::prelude::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, glib::Enum, Default)]
#[enum_type(name = "PkSetupKind")]
pub enum SetupAction {
    #[default]
    Init,
    AddExisting,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, glib::Enum, Default)]
#[enum_type(name = "PkSetupRepoKind")]
pub enum SetupLocationKind {
    #[default]
    Local,
    Remote,
}

#[derive(Debug, Clone, glib::Boxed)]
#[boxed_type(name = "PkSetupRepoLocation", nullable)]
pub enum SetupRepoLocation {
    Local(gio::File),
    Remote(String),
}

impl SetupRepoLocation {
    /// Takes a [`gio::File`]. Any checks will be performed when creating the
    /// repo config
    pub fn from_file(file: gio::File) -> Self {
        Self::Local(file)
    }

    /// Parse borg URIs and canonicalize them to the `ssh://` syntax.
    /// All other URIs are being taken verbatim
    pub fn parse_url(input: String) -> std::result::Result<Self, String> {
        let url = if !input.contains("://") {
            if let Some((target, path)) = input.split_once(':') {
                let path_begin = path.chars().next();

                let url_path = if path_begin == Some('~') {
                    format!("/{path}")
                } else if path_begin != Some('/') {
                    format!("/./{path}")
                } else {
                    path.to_string()
                };

                format!("ssh://{target}{url_path}")
            } else {
                return Err(gettext("Incomplete URL or borg syntax"));
            }
        } else {
            input
        };

        match glib::Uri::parse(&url, glib::UriFlags::NONE) {
            Ok(uri) => {
                if uri.path().is_empty() {
                    return Err(gettext("The remote location must have a specified path."));
                }

                if uri.scheme() == "ssh" {
                    Ok(Self::Remote(url))
                } else {
                    Ok(Self::Local(gio::File::for_uri(&url)))
                }
            }
            Err(err) => Err(gettextf("Invalid remote location: “{}”", [err.message()])),
        }
    }
}

impl std::fmt::Display for SetupRepoLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetupRepoLocation::Local(file) => write!(f, "{}", file.uri()),
            SetupRepoLocation::Remote(uri) => write!(f, "{}", uri),
        }
    }
}

#[derive(Debug, Default, Clone, glib::Boxed, PartialEq, Eq)]
#[boxed_type(name = "PkSetupCommandLineArgs", nullable)]
pub struct SetupCommandLineArgs(Vec<String>);

impl FromStr for SetupCommandLineArgs {
    type Err = crate::error::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self(crate::utils::borg::parse_borg_command_line_args(s)?))
    }
}

impl SetupCommandLineArgs {
    pub const NONE: Self = Self(Vec::new());

    pub fn into_inner(self) -> Vec<String> {
        self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Display for SetupCommandLineArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", shell_words::join(&self.0))
    }
}

#[derive(Clone, Debug, glib::Boxed)]
#[boxed_type(name = "PkSetupArchiveParams", nullable)]
pub struct ArchiveParams {
    pub prefix: Option<config::ArchivePrefix>,
    pub parsed: borg::invert_command::Parsed,
    pub hostname: String,
    pub username: String,
    pub end: chrono::NaiveDateTime,
    pub stats: borg::json::Stats,
}

impl From<borg::ListArchive> for ArchiveParams {
    fn from(archive: borg::ListArchive) -> Self {
        let prefix = archive
            .name
            .as_str()
            .split_once('-')
            .map(|x| config::ArchivePrefix(x.0.to_string() + "-"));
        let stats = borg::json::Stats::transfer_history_mock(&archive);
        let parsed = borg::invert_command::parse(archive.command_line);

        ArchiveParams {
            prefix,
            parsed,
            hostname: archive.hostname,
            username: archive.username,
            end: archive.end,
            stats,
        }
    }
}
