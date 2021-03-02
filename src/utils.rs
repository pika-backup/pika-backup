use crate::prelude::*;

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
