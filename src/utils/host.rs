use std::path::PathBuf;

pub fn user_data_dir() -> PathBuf {
    if *crate::globals::APP_IS_SANDBOXED {
        match std::env::var_os("HOST_XDG_DATA_HOME") {
            Some(dir) if !dir.is_empty() => dir.into(),
            _ => glib::home_dir().join(".local/share"),
        }
    } else {
        glib::user_data_dir()
    }
}

pub fn user_cache_dir() -> PathBuf {
    if *crate::globals::APP_IS_SANDBOXED {
        match std::env::var_os("HOST_XDG_CACHE_HOME") {
            Some(dir) if !dir.is_empty() => dir.into(),
            _ => glib::home_dir().join(".cache"),
        }
    } else {
        glib::user_cache_dir()
    }
}

pub fn user_config_dir() -> PathBuf {
    if *crate::globals::APP_IS_SANDBOXED {
        match std::env::var_os("HOST_XDG_CONFIG_HOME") {
            Some(dir) if !dir.is_empty() => dir.into(),
            _ => glib::home_dir().join(".config"),
        }
    } else {
        glib::user_config_dir()
    }
}

pub fn user_runtime_dir() -> PathBuf {
    // TODO: HOST_XDG_RUNTIME_DIR doesn't exist, we just assume it's the same on
    // host and inside flatpak See: https://github.com/flatpak/flatpak/issues/4885
    glib::user_runtime_dir()
}
