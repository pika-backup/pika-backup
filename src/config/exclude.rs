use super::*;

use once_cell::sync::Lazy;
use std::ffi::OsString;
use std::io::Read;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(untagged)]
pub enum Exclude<const T: Relativity> {
    Pattern(Pattern<T>),
    Predefined(Predefined),
}

impl Exclude<{ ABSOLUTE }> {
    pub fn into_relative(self) -> Exclude<{ RELATIVE }> {
        match self {
            Self::Pattern(p) => Exclude::Pattern(p.into_relative()),
            Self::Predefined(x) => Exclude::Predefined(x),
        }
    }
}

impl Exclude<{ RELATIVE }> {
    pub fn into_absolute(self) -> Exclude<{ ABSOLUTE }> {
        match self {
            Self::Pattern(p) => Exclude::Pattern(p.into_absolute()),
            Self::Predefined(x) => Exclude::Predefined(x),
        }
    }
}

impl<const T: Relativity> Exclude<T> {
    pub fn from_pattern(pattern: Pattern<T>) -> Self {
        Self::Pattern(pattern)
    }

    pub fn from_predefined(predefined: Predefined) -> Self {
        Self::Predefined(predefined)
    }

    pub fn is_predefined(&self) -> bool {
        matches!(self, Self::Predefined(_))
    }

    pub fn borg_rules(&self) -> Vec<BorgRule> {
        match self {
            Self::Pattern(pattern) => vec![BorgRule::Pattern(pattern.borg_pattern())],
            Self::Predefined(predefined) => predefined.borg_rules(),
        }
    }

    pub fn is_match(&self, path: &std::path::Path) -> bool {
        match self {
            Self::Pattern(pattern) => pattern.is_match(path),
            Self::Predefined(predefined) => predefined.rules().iter().any(|rule| match rule {
                Rule::Pattern(pattern) => pattern.is_match(path),
                Rule::CacheDirTag => path_is_cachedir(path),
            }),
        }
    }

    pub fn symbolic_icon(&self) -> Option<gtk::Image> {
        match self {
            Self::Pattern(pattern) => pattern.symbolic_icon(),
            Self::Predefined(predefined) => Some(predefined.symbolic_icon()),
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::Pattern(pattern) => pattern.description(),
            Self::Predefined(predefined) => predefined.description(),
        }
    }

    pub fn kind(&self) -> String {
        match self {
            Self::Pattern(pattern) => pattern.kind(),
            Self::Predefined(predefined) => predefined.kind(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Predefined {
    Caches,
    Trash,
    FlatpakApps,
    VmsContainers,
}

mod rules {
    use super::*;

    pub static CACHES: Lazy<[Rule<ABSOLUTE>; 3]> = Lazy::new(|| {
        [
            // XDG cache
            Rule::Pattern(Pattern::PathPrefix(crate::utils::host::user_cache_dir())),
            // Flatpak app caches
            Rule::Pattern(Pattern::RegularExpression(
                regex::Regex::new(&format!(
                    r"^{}/\.var/app/[^/]+/cache/",
                    borg_regex_path(&glib::home_dir())
                ))
                .unwrap(),
            )),
            // CACHEDIR.TAG
            Rule::CacheDirTag,
        ]
    });

    pub static VMS_CONTAINERS: Lazy<[Rule<ABSOLUTE>; 9]> = Lazy::new(|| {
        [
            // Boxes (host)
            Rule::Pattern(Pattern::PathPrefix(
                crate::utils::host::user_data_dir().join("gnome-boxes"),
            )),
            // Boxes (flatpak)
            Rule::Pattern(Pattern::PathPrefix(
                glib::home_dir().join(".var/app/org.gnome.Boxes"),
            )),
            Rule::Pattern(Pattern::PathPrefix(
                glib::home_dir().join(".var/app/org.gnome.BoxesDevel"),
            )),
            // Bottles (host)
            Rule::Pattern(Pattern::PathPrefix(
                crate::utils::host::user_data_dir().join("bottles"),
            )),
            // Bottles (flatpak)
            Rule::Pattern(Pattern::PathPrefix(
                glib::home_dir().join(".var/app/com.usebottles.bottles"),
            )),
            // libvirt
            Rule::Pattern(Pattern::PathPrefix(
                crate::utils::host::user_data_dir().join("libvirt"),
            )),
            // stores libvirt snapshots etc
            Rule::Pattern(Pattern::PathPrefix(
                crate::utils::host::user_config_dir().join("libvirt"),
            )),
            // podman/toolbox
            Rule::Pattern(Pattern::PathPrefix(
                crate::utils::host::user_data_dir().join("containers"),
            )),
            // docker
            Rule::Pattern(Pattern::PathPrefix(
                crate::utils::host::user_data_dir().join("docker"),
            )),
        ]
    });

    pub static FLATPAK_APPS: Lazy<[Rule<ABSOLUTE>; 1]> = Lazy::new(|| {
        [Rule::Pattern(Pattern::RegularExpression(
            regex::Regex::new(&format!(
                r"^{}/flatpak/(?!overrides)",
                borg_regex_path(&crate::utils::host::user_data_dir())
            ))
            .unwrap(),
        ))]
    });

    pub static TRASH: Lazy<[Rule<ABSOLUTE>; 1]> = Lazy::new(|| {
        [Rule::Pattern(Pattern::PathPrefix(
            crate::utils::host::user_data_dir().join("Trash"),
        ))]
    });
}

impl Predefined {
    pub const VALUES: [Self; 4] = [
        Self::Caches,
        Self::Trash,
        Self::FlatpakApps,
        Self::VmsContainers,
    ];

    pub fn symbolic_icon(&self) -> gtk::Image {
        match self {
            Self::Trash => gtk::Image::from_icon_name("user-trash-symbolic"),
            Self::VmsContainers => gtk::Image::from_icon_name("computer-symbolic"),
            Self::FlatpakApps => gtk::Image::from_icon_name("preferences-desktop-apps-symbolic"),
            _ => gtk::Image::from_icon_name("folder-saved-search-symbolic"),
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::Caches => gettext("Caches"),
            Self::FlatpakApps => gettext("Flatpak App Installations"),
            Self::Trash => gettext("Trash"),
            Self::VmsContainers => gettext("Virtual Machines and Containers"),
        }
    }

    pub fn kind(&self) -> String {
        match self {
            // Translators: Detailed description for caches exclusion rule
            Self::Caches => gettext("Data that can be regenerated when needed"),
            // Translators: Detailed description for Flatpak app installations exclusion rule
            Self::FlatpakApps => gettext("Documents and data are still backed up"),
            // Translators: Detailed description for trash exclusion rule
            Self::Trash => gettext("Files that have not been irretrievably deleted"),
            // Translators: Detailed description for virtual machines and containers exclusion rule
            Self::VmsContainers => gettext("Might include data stored within"),
        }
    }

    pub fn rules(&self) -> &[Rule<ABSOLUTE>] {
        match self {
            Self::Caches => rules::CACHES.as_ref(),
            Self::FlatpakApps => rules::FLATPAK_APPS.as_ref(),
            Self::Trash => rules::TRASH.as_ref(),
            Self::VmsContainers => rules::VMS_CONTAINERS.as_ref(),
        }
    }

    pub fn borg_rules(&self) -> Vec<BorgRule> {
        self.rules().iter().map(BorgRule::from).collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rule<const T: Relativity> {
    Pattern(Pattern<T>),
    /// Directories that have a CACHEDIR.TAG
    ///
    /// <https://bford.info/cachedir/>
    CacheDirTag,
}

#[derive(Debug)]
pub enum BorgRule {
    Pattern(OsString),
    CacheDirTag,
}

impl From<Rule<ABSOLUTE>> for BorgRule {
    fn from(rule: Rule<ABSOLUTE>) -> Self {
        match rule {
            Rule::Pattern(pattern) => BorgRule::Pattern(pattern.borg_pattern()),
            Rule::CacheDirTag => BorgRule::CacheDirTag,
        }
    }
}

impl From<&Rule<ABSOLUTE>> for BorgRule {
    fn from(rule: &Rule<ABSOLUTE>) -> Self {
        match rule {
            Rule::Pattern(pattern) => BorgRule::Pattern(pattern.borg_pattern()),
            Rule::CacheDirTag => BorgRule::CacheDirTag,
        }
    }
}

impl std::fmt::Display for BorgRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pattern(pattern) => {
                write!(f, "{}", pattern.clone().into_string().unwrap_or_default())
            }
            Self::CacheDirTag => write!(f, "CACHEDIR.TAG"),
        }
    }
}

fn borg_regex_path(path: &Path) -> String {
    // TODO: many unwraps
    regex::escape(path.strip_prefix("/").unwrap().to_str().unwrap()).to_string()
}

/// <https://bford.info/cachedir/>
pub const CACHEDIR_TAG_HEADER: &[u8; 43] = b"Signature: 8a477f597d28d172789f06886806bc55";

fn path_is_cachedir(directory: &std::path::Path) -> bool {
    if let Ok(mut file) = std::fs::File::open(directory.join("CACHEDIR.TAG")) {
        let mut buffer = [0; CACHEDIR_TAG_HEADER.len()];
        let _ignore = file.read(&mut buffer);
        CACHEDIR_TAG_HEADER == &buffer
    } else {
        false
    }
}
