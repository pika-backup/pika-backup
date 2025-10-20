use std::collections::HashMap;

use serde::Deserialize;

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct FlatpakInfo {
    instance: Instance,
    context: HashMap<String, String>,
    #[serde(rename = "Session Bus Policy")]
    session_bus_policy: HashMap<String, String>,
    #[serde(rename = "System Bus Policy")]
    system_bus_policy: HashMap<String, String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct Instance {
    app_commit: Option<String>,
    flatpak_version: Option<String>,
}

pub fn get() -> Result<FlatpakInfo, Box<dyn std::error::Error>> {
    let file = std::fs::File::open("/.flatpak-info")?;
    let info: FlatpakInfo = serde_ini::de::from_read(&file)?;
    Ok(info)
}
