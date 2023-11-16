use crate::prelude::*;

use super::{absolute, display_path};
use serde::Deserialize;
use std::ffi::{CString, OsString};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

pub const RELATIVE: bool = false;
pub const ABSOLUTE: bool = true;

pub type Relativity = bool;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Pattern<const T: Relativity> {
    Fnmatch(OsString),
    PathFullMatch(PathBuf),
    PathPrefix(PathBuf),
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

impl<const T: Relativity> std::cmp::PartialEq for Pattern<T> {
    fn eq(&self, other: &Self) -> bool {
        self.borg_pattern() == other.borg_pattern()
    }
}
impl<const T: Relativity> std::cmp::Eq for Pattern<T> {}

impl<const T: Relativity> std::cmp::Ord for Pattern<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.borg_pattern().cmp(&other.borg_pattern())
    }
}

impl<const T: Relativity> std::cmp::PartialOrd for Pattern<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<const T: Relativity> std::hash::Hash for Pattern<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.borg_pattern().hash(state);
    }
}

/// Returns a relative path for sub directories of home
pub fn rel_path(path: PathBuf) -> PathBuf {
    if let Ok(rel_path) = path.strip_prefix(glib::home_dir().as_path()) {
        rel_path.to_path_buf()
    } else {
        path
    }
}

impl Pattern<{ RELATIVE }> {
    pub fn into_absolute(self) -> Pattern<{ ABSOLUTE }> {
        match self {
            Self::Fnmatch(x) => Pattern::Fnmatch(x),
            Self::PathPrefix(path) => Pattern::PathPrefix(absolute(&path)),
            Self::PathFullMatch(path) => Pattern::PathPrefix(absolute(&path)),
            Self::RegularExpression(x) => Pattern::RegularExpression(x),
        }
    }
}

impl Pattern<{ ABSOLUTE }> {
    pub fn into_relative(self) -> Pattern<{ RELATIVE }> {
        match self {
            Self::Fnmatch(x) => Pattern::Fnmatch(x),
            Self::PathPrefix(path) => Pattern::PathPrefix(rel_path(path)),
            Self::PathFullMatch(path) => Pattern::PathPrefix(rel_path(path)),
            Self::RegularExpression(x) => Pattern::RegularExpression(x),
        }
    }

    pub fn from_borg(s: String) -> Option<Self> {
        if let Some((selector, pattern)) = s.split_once(':') {
            match selector {
                "fm" => Some(Self::Fnmatch(OsString::from(pattern))),
                "pp" => Some(Self::PathPrefix(
                    PathBuf::from(pattern)
                        .strip_prefix(glib::home_dir())
                        .map(|x| x.to_path_buf())
                        .unwrap_or_else(|_| pattern.into()),
                )),
                "re" => regex::Regex::new(pattern).map(Self::RegularExpression).ok(),
                "pf" => Some(Self::PathFullMatch(
                    PathBuf::from(pattern)
                        .strip_prefix(glib::home_dir())
                        .map(|x| x.to_path_buf())
                        .unwrap_or_else(|_| pattern.into()),
                )),
                _ => None,
            }
        } else if s.contains(['*', '?', '[']) {
            Some(Self::Fnmatch(OsString::from(s)))
        } else {
            Some(Self::PathPrefix(
                PathBuf::from(&s)
                    .strip_prefix(glib::home_dir())
                    .map(|x| x.to_path_buf())
                    .unwrap_or_else(|_| s.into()),
            ))
        }
    }
}

impl<const T: bool> Pattern<T> {
    pub fn fnmatch(pattern: impl Into<OsString>) -> Self {
        Self::Fnmatch(pattern.into())
    }

    pub fn path_prefix(path: impl Into<PathBuf>) -> Self {
        let path = match T {
            ABSOLUTE => absolute(&path.into()),
            RELATIVE => rel_path(path.into()),
        };

        Self::PathPrefix(path)
    }

    pub fn path_full_match(path: impl Into<PathBuf>) -> Self {
        let path = match T {
            ABSOLUTE => absolute(&path.into()),
            RELATIVE => rel_path(path.into()),
        };

        Self::PathFullMatch(path)
    }

    pub fn from_regular_expression(re: impl AsRef<str>) -> Result<Self, regex::Error> {
        Ok(Self::RegularExpression(regex::Regex::new(re.as_ref())?))
    }

    ///
    /// ```
    /// # use pika_backup::config;
    /// # type Pattern = config::Pattern<{config::ABSOLUTE}>;
    /// let path = std::path::Path::new("/tmp/test/file");
    ///
    /// assert!(Pattern::fnmatch("*/test/").is_match(path));
    /// assert!(Pattern::fnmatch("tmp/test/").is_match(path));
    /// assert!(Pattern::fnmatch("/tmp/test/").is_match(path));
    /// assert!(Pattern::fnmatch("t*st").is_match(path));
    ///
    /// assert!(!Pattern::fnmatch("/test/").is_match(path));
    /// assert!(!Pattern::fnmatch("xxx").is_match(path));
    /// ```
    pub fn is_match(&self, path: &Path) -> bool {
        match self {
            Self::Fnmatch(pattern) => {
                let mut bytes = pattern.clone().into_vec();
                if let Some(stripped) = bytes.strip_prefix(b"/") {
                    bytes = stripped.to_vec();
                }
                bytes.push(b'*');

                let mut path = path.to_path_buf();
                if let Ok(stripped) = path.strip_prefix("/") {
                    path = stripped.to_path_buf();
                }

                if let (Ok(pattern), Ok(path)) = (
                    CString::new(bytes),
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
            Self::Fnmatch(_) => gettext("Unix Filename Pattern"),
        }
    }

    pub fn symbolic_icon(&self) -> Option<gtk::Image> {
        match self {
            Self::PathPrefix(path) | Self::PathFullMatch(path) => {
                crate::utils::file_symbolic_icon(&absolute(path))
            }
            Self::Fnmatch(_) | Self::RegularExpression(_) => {
                Some(gtk::Image::from_icon_name("folder-saved-search-symbolic"))
            }
        }
    }
}
