use ashpd::flatpak;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::io::Read;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::io::AsRawFd;
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

pub fn user_runtime_dir() -> PathBuf {
    if ashpd::is_sandboxed() {
        match HOST_XDG_RUNTIME_DIR.as_ref() {
            Ok(dir) if dir.is_absolute() => dir.to_path_buf(),
            _ => glib::user_runtime_dir(),
        }
    } else {
        glib::user_runtime_dir()
    }
}

static HOST_XDG_RUNTIME_DIR: Lazy<Result<PathBuf, Box<dyn std::error::Error + Send + Sync>>> =
    Lazy::new(|| async_std::task::block_on(host_runtime_dir()));

async fn host_runtime_dir() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let (mut pipe_reader, pipe_writer) = std::os::unix::net::UnixStream::pair()?;

    let connection = ashpd::zbus::Connection::session().await?;
    let proxy = flatpak::FlatpakProxy::new(&connection).await?;
    proxy
        .spawn(
            glib::home_dir(),
            &["printenv", "XDG_RUNTIME_DIR"],
            HashMap::from([(1, pipe_writer.as_raw_fd().into())]),
            Default::default(),
            Default::default(),
            Default::default(),
        )
        .await?;
    drop(pipe_writer);

    let mut res = Vec::new();
    pipe_reader.read_to_end(&mut res)?;

    if res.ends_with(b"\n") {
        res.pop();
    }

    let path = PathBuf::from(std::ffi::OsString::from_vec(res));

    debug!("XDG_RUNTIME_DIR reply from host: {:?}", path);

    Ok(path)
}
