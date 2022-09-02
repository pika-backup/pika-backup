use std::path::PathBuf;

pub fn user_data_dir() -> PathBuf {
    if ashpd::is_sandboxed() {
        match std::env::var_os("HOST_XDG_DATA_HOME") {
            Some(dir) if !dir.is_empty() => dir.into(),
            _ => glib::home_dir().join(".local/share"),
        }
    } else {
        glib::user_data_dir()
    }
}

pub fn user_cache_dir() -> PathBuf {
    if ashpd::is_sandboxed() {
        match std::env::var_os("HOST_XDG_CACHE_HOME") {
            Some(dir) if !dir.is_empty() => dir.into(),
            _ => glib::home_dir().join(".cache"),
        }
    } else {
        glib::user_cache_dir()
    }
}

pub fn user_config_dir() -> PathBuf {
    if ashpd::is_sandboxed() {
        match std::env::var_os("HOST_XDG_CONFIG_HOME") {
            Some(dir) if !dir.is_empty() => dir.into(),
            _ => glib::home_dir().join(".config"),
        }
    } else {
        glib::user_config_dir()
    }
}
