use super::*;

use once_cell::sync::Lazy;
use std::ffi::OsString;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(untagged)]
pub enum Exclude {
    Pattern(Pattern),
    Predefined(Predefined),
}

impl Exclude {
    pub fn from_pattern(pattern: Pattern) -> Self {
        Self::Pattern(pattern)
    }

    pub fn from_predefined(predefined: Predefined) -> Self {
        Self::Predefined(predefined)
    }

    pub fn is_predefined(&self) -> bool {
        matches!(self, Self::Predefined(_))
    }

    pub fn borg_patterns(&self) -> Vec<OsString> {
        match self {
            Self::Pattern(pattern) => vec![pattern.borg_pattern()],
            Self::Predefined(predefined) => predefined.borg_patterns(),
        }
    }

    pub fn is_match(&self, path: &std::path::Path) -> bool {
        match self {
            Self::Pattern(pattern) => pattern.is_match(path),
            Self::Predefined(predefined) => predefined.patterns().iter().any(|x| x.is_match(path)),
        }
    }

    pub fn icon(&self) -> Option<gio::Icon> {
        match self {
            Self::Pattern(pattern) => pattern.symbolic_icon(),
            Self::Predefined(predefined) => predefined.icon(),
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
    FlatpakApps,
    Trash,
    VmsContainers,
}

mod patterns {
    use super::*;

    pub static CACHES: Lazy<[Pattern; 2]> = Lazy::new(|| {
        [
            // XDG cache
            Pattern::PathPrefix(crate::utils::host::user_cache_dir()),
            // Flatpak app caches
            Pattern::RegularExpression(
                regex::Regex::new(&format!(
                    r"^{}/\.var/app/[^/]+/cache/",
                    borg_regex_path(&glib::home_dir())
                ))
                .unwrap(),
            ),
        ]
    });

    pub static VMS_CONTAINERS: Lazy<[Pattern; 8]> = Lazy::new(|| {
        [
            // Boxes (host)
            Pattern::PathPrefix(crate::utils::host::user_data_dir().join("gnome-boxes")),
            // Boxes (flatpak)
            Pattern::PathPrefix(glib::home_dir().join(".var/app/org.gnome.Boxes")),
            Pattern::PathPrefix(glib::home_dir().join(".var/app/org.gnome.BoxesDevel")),
            // Bottles (host)
            Pattern::PathPrefix(crate::utils::host::user_data_dir().join("bottles")),
            // Bottles (flatpak)
            Pattern::PathPrefix(glib::home_dir().join(".var/app/com.usebottles.bottles")),
            // libvirt
            Pattern::PathPrefix(crate::utils::host::user_data_dir().join("libvirt")),
            // stores libvirt snapshots etc
            Pattern::PathPrefix(crate::utils::host::user_config_dir().join("libvirt")),
            // podman/toolbox
            Pattern::PathPrefix(crate::utils::host::user_data_dir().join("containers")),
        ]
    });

    pub static FLATPAK_APPS: Lazy<[Pattern; 1]> = Lazy::new(|| {
        [Pattern::RegularExpression(
            regex::Regex::new(&format!(
                r"^{}/flatpak/(?!overrides)",
                borg_regex_path(&crate::utils::host::user_data_dir())
            ))
            .unwrap(),
        )]
    });

    pub static TRASH: Lazy<[Pattern; 1]> =
        Lazy::new(|| [Pattern::PathPrefix(glib::user_data_dir().join("Trash"))]);
}

impl Predefined {
    pub const VALUES: [Self; 4] = [
        Self::Caches,
        Self::FlatpakApps,
        Self::Trash,
        Self::VmsContainers,
    ];

    pub fn icon(&self) -> Option<gio::Icon> {
        match self {
            Self::Trash => gio::Icon::for_string("user-trash-symbolic").ok(),
            Self::VmsContainers => gio::Icon::for_string("computer-symbolic").ok(),
            Self::FlatpakApps => gio::Icon::for_string("preferences-desktop-apps-symbolic").ok(),
            _ => gio::Icon::for_string("folder-saved-search-symbolic").ok(),
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
            Self::Caches => gettext("Data that can be regenerated when needed"),
            Self::FlatpakApps => gettext("Documents and data are still backed up"),
            Self::Trash => String::new(),
            Self::VmsContainers => String::new(),
        }
    }

    pub fn patterns(&self) -> &[Pattern] {
        match self {
            Self::Caches => patterns::CACHES.as_ref(),
            Self::FlatpakApps => patterns::FLATPAK_APPS.as_ref(),
            Self::Trash => patterns::TRASH.as_ref(),
            Self::VmsContainers => patterns::VMS_CONTAINERS.as_ref(),
        }
    }

    pub fn borg_patterns(&self) -> Vec<OsString> {
        self.patterns().iter().map(|x| x.borg_pattern()).collect()
    }
}

fn borg_regex_path(path: &Path) -> String {
    // TODO: many unwraps
    regex::escape(path.strip_prefix("/").unwrap().to_str().unwrap()).to_string()
}
