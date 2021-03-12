use crate::config;
use crate::prelude::*;

use gio::prelude::*;

pub fn prepare_config_file<V: serde::Serialize>(
    filename: &str,
    default_value: V,
) -> std::io::Result<std::path::PathBuf> {
    let mut path = CONFIG_DIR.clone();
    path.push(env!("CARGO_PKG_NAME"));
    std::fs::create_dir_all(&path)?;
    path.push(filename);

    if let Ok(file) = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        serde_json::ser::to_writer_pretty(file, &default_value)?;
    }

    Ok(path)
}

pub trait LookupConfigId<T> {
    fn get_mut_result(&mut self, key: &ConfigId) -> Result<&mut T, config::error::BackupNotFound>;
    fn get_result(&self, key: &ConfigId) -> Result<&T, config::error::BackupNotFound>;
}

pub fn get_mount_uuid(mount: &gio::Mount) -> Option<String> {
    let volume = mount.get_volume();

    volume
        .as_ref()
        .and_then(gio::Volume::get_uuid)
        .or_else(|| volume.as_ref().and_then(|v| v.get_identifier("uuid")))
        .as_ref()
        .map(std::string::ToString::to_string)
}