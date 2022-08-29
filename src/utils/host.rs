use std::path::PathBuf;

// TODO: The flatpak versions are wrong

pub fn user_data_dir() -> PathBuf {
    match std::env::var_os("HOST_XDG_DATA_HOME") {
        Some(dir) if ashpd::is_sandboxed() => dir.into(),
        _ => glib::user_data_dir(),
    }
}

pub fn user_cache_dir() -> PathBuf {
    match std::env::var_os("HOST_XDG_CACHE_HOME") {
        Some(dir) if ashpd::is_sandboxed() => dir.into(),
        _ => glib::user_cache_dir(),
    }
}

pub fn user_config_dir() -> PathBuf {
    match std::env::var_os("HOST_XDG_CONFIG_HOME") {
        Some(dir) if ashpd::is_sandboxed() => dir.into(),
        _ => glib::user_config_dir(),
    }
}
