use crate::prelude::*;

use super::{absolute, display_path};
use serde::Deserialize;
use std::ffi::CString;
use std::ffi::OsString;
use std::os::unix::ffi::OsStrExt;
use std::path;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Pattern {
    Fnmatch(OsString),
    PathFullMatch(path::PathBuf),
    PathPrefix(path::PathBuf),
    #[serde(
        deserialize_with = "deserialize_regex",
        serialize_with = "serialize_regex"
    )]
    RegularExpression(regex::Regex),
}

fn deserialize_regex<'de, D>(deserializer: D) -> Result<regex::Regex, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let string = String::deserialize(deserializer)?;
    regex::Regex::new(&string).map_err(serde::de::Error::custom)
}

fn serialize_regex<S>(regex: &regex::Regex, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_str(regex.as_str())
}

impl std::cmp::PartialEq for Pattern {
    fn eq(&self, other: &Self) -> bool {
        self.borg_pattern() == other.borg_pattern()
    }
}
impl std::cmp::Eq for Pattern {}

impl std::cmp::Ord for Pattern {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.borg_pattern().cmp(&other.borg_pattern())
    }
}

impl std::cmp::PartialOrd for Pattern {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::hash::Hash for Pattern {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.borg_pattern().hash(state);
    }
}

impl Pattern {
    pub fn from_borg(s: String) -> Option<Self> {
        if let Some((selector, pattern)) = s.split_once(':') {
            match selector {
                "fm" => Some(Self::Fnmatch(std::ffi::OsString::from(pattern))),
                "pp" => Some(Self::PathPrefix(
                    path::PathBuf::from(pattern)
                        .strip_prefix(glib::home_dir())
                        .map(|x| x.to_path_buf())
                        .unwrap_or_else(|_| pattern.into()),
                )),
                "re" => regex::Regex::new(pattern).map(Self::RegularExpression).ok(),
                "pf" => Some(Self::PathFullMatch(
                    path::PathBuf::from(pattern)
                        .strip_prefix(glib::home_dir())
                        .map(|x| x.to_path_buf())
                        .unwrap_or_else(|_| pattern.into()),
                )),
                _ => None,
            }
        } else if s.contains(['*', '?', '[']) {
            Some(Self::Fnmatch(std::ffi::OsString::from(s)))
        } else {
            Some(Self::PathPrefix(
                path::PathBuf::from(&s)
                    .strip_prefix(glib::home_dir())
                    .map(|x| x.to_path_buf())
                    .unwrap_or_else(|_| s.into()),
            ))
        }
    }

    pub fn is_match(&self, path: &std::path::Path) -> bool {
        match self {
            Self::Fnmatch(pattern) => {
                if let (Ok(pattern), Ok(path)) = (
                    CString::new(pattern.as_bytes()),
                    CString::new(path.as_os_str().as_bytes()),
                ) {
                    crate::utils::posix_fnmatch(&pattern, &path)
                } else {
                    false
                }
            }
            Self::PathPrefix(path_prefix) => path.starts_with(absolute(path_prefix)),
            // TODO: warn if fail
            Self::RegularExpression(regex) => {
                let mut path_ = path.to_string_lossy().to_string();
                if let Some(unprefixed) = path_.strip_prefix('/') {
                    path_ = unprefixed.to_string();
                }
                regex.is_match(&path_).unwrap_or_default()
            }
            Self::PathFullMatch(full_path) => path == absolute(full_path),
        }
    }

    pub fn selector(&self) -> String {
        match self {
            Self::Fnmatch(_) => "fm",
            Self::PathPrefix(_) => "pp",
            Self::RegularExpression(_) => "re",
            Self::PathFullMatch(_) => "pf",
        }
        .to_string()
    }

    pub fn pattern(&self) -> OsString {
        match self {
            Self::Fnmatch(pattern) => pattern.into(),
            Self::PathPrefix(path) | Self::PathFullMatch(path) => absolute(path).into(),
            Self::RegularExpression(regex) => regex.as_str().into(),
        }
    }

    // TODO: shouldn't this be OsString?
    pub fn borg_pattern(&self) -> OsString {
        let mut pattern = OsString::from(self.selector());
        pattern.push(":");
        pattern.push(self.pattern());

        pattern
    }

    pub fn description(&self) -> String {
        match self {
            Self::Fnmatch(pattern) => pattern.to_string_lossy().to_string(),
            Self::PathPrefix(path) | Self::PathFullMatch(path) => display_path(path),
            Self::RegularExpression(regex) => regex.to_string(),
        }
    }

    pub fn kind(&self) -> String {
        match self {
            Self::PathPrefix(_) | Self::PathFullMatch(_) => String::new(),
            Self::RegularExpression(_) => gettext("Regular Expression"),
            Self::Fnmatch(_) => gettext("Shell Wildcard Pattern"),
        }
    }

    pub fn icon(&self) -> Option<gio::Icon> {
        match self {
            Self::PathPrefix(path) | Self::PathFullMatch(path) => {
                crate::utils::file_icon(&absolute(path))
            }
            Self::Fnmatch(_) | Self::RegularExpression(_) => {
                gio::Icon::for_string("folder-saved-search").ok()
            }
        }
    }

    pub fn symbolic_icon(&self) -> Option<gio::Icon> {
        match self {
            Self::PathPrefix(path) | Self::PathFullMatch(path) => {
                crate::utils::file_symbolic_icon(&absolute(path))
            }
            Self::Fnmatch(_) | Self::RegularExpression(_) => {
                gio::Icon::for_string("folder-saved-search-symbolic").ok()
            }
        }
    }
}
