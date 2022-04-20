use crate::prelude::*;

use super::{absolute, display_path};
use serde::Deserialize;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Pattern {
    Fnmatch(std::ffi::OsString),
    PathPrefix(path::PathBuf),
    #[serde(
        deserialize_with = "deserialize_regex",
        serialize_with = "serialize_regex"
    )]
    RegularExpression(Box<regex::Regex>),
}

fn deserialize_regex<'de, D>(deserializer: D) -> Result<Box<regex::Regex>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let string = String::deserialize(deserializer)?;
    regex::Regex::new(&string)
        .map(Box::new)
        .map_err(serde::de::Error::custom)
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
                "re" => regex::Regex::new(pattern)
                    .map(Box::new)
                    .map(Self::RegularExpression)
                    .ok(),
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

    pub fn cache() -> Self {
        Self::PathPrefix(".cache".into())
    }

    pub fn flatpak_app_cache() -> Self {
        Self::RegularExpression(Box::new(
            regex::Regex::new(r"/\.var/app/[^/]+/cache/").unwrap(),
        ))
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
            Self::PathPrefix(path_prefix) => path.starts_with(path_prefix),
            Self::RegularExpression(regex) => regex.is_match(&path.to_string_lossy()),
        }
    }

    pub fn selector(&self) -> String {
        match self {
            Self::Fnmatch(_) => "fm",
            Self::PathPrefix(_) => "pp",
            Self::RegularExpression(_) => "re",
        }
        .to_string()
    }

    pub fn pattern(&self) -> String {
        match self {
            Self::Fnmatch(pattern) => pattern.to_string_lossy().to_string(),
            Self::PathPrefix(path) => path.to_string_lossy().to_string(),
            Self::RegularExpression(regex) => regex.as_str().to_string(),
        }
    }

    // TODO: shouldn't this be OsString?
    pub fn borg_pattern(&self) -> String {
        format!("{}:{}", self.selector(), self.pattern())
    }

    pub fn description(&self) -> String {
        match self {
            pattern if pattern == &Self::flatpak_app_cache() => gettext("Flatpak App Cache"),
            Self::Fnmatch(pattern) => pattern.to_string_lossy().to_string(),
            Self::PathPrefix(path) => display_path(path),
            Self::RegularExpression(regex) => regex.to_string(),
        }
    }

    pub fn icon(&self) -> Option<gio::Icon> {
        match self {
            Self::PathPrefix(path) => crate::utils::file_icon(&absolute(path)),
            Self::Fnmatch(_) | Self::RegularExpression(_) => {
                gio::Icon::for_string("folder-saved-search").ok()
            }
        }
    }

    pub fn symbolic_icon(&self) -> Option<gio::Icon> {
        match self {
            Self::PathPrefix(path) => crate::utils::file_symbolic_icon(&absolute(path)),
            Self::Fnmatch(_) | Self::RegularExpression(_) => {
                gio::Icon::for_string("folder-saved-search-symbolic").ok()
            }
        }
    }
}
