use gio::prelude::*;

use super::{BackupSettings, local, remote};
use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, glib::Boxed)]
#[boxed_type(name = "PkRepositoryConfig", nullable)]
#[serde(tag = "type")]
pub enum Repository {
    Local(local::Repository),
    Remote(remote::Repository),
}

async fn ssh_host_lookup(host: &str) -> String {
    let result = async_process::Command::new("ssh")
        .args(["-G", host])
        .stdout(async_process::Stdio::piped())
        .output()
        .await;

    match result {
        Err(err) => {
            warn!("SSH config lookup failed: {}", err);
            host.to_string()
        }
        Ok(output) => String::from_utf8_lossy(&output.stdout)
            .lines()
            .find_map(|x| x.strip_prefix("hostname "))
            .unwrap_or(host)
            .to_string(),
    }
}

impl Repository {
    pub async fn host(&self) -> Option<String> {
        match self {
            Self::Local(local) => {
                let uri = glib::Uri::parse(local.uri.as_ref()?, glib::UriFlags::NONE).ok()?;

                if ["sftp", "ssh"].contains(&uri.scheme().as_str()) {
                    match uri.host() {
                        Some(host) => Some(ssh_host_lookup(&host).await),
                        _ => None,
                    }
                } else {
                    uri.host().as_deref().map(str::to_string)
                }
            }
            Self::Remote(remote) => {
                match glib::Uri::parse(&remote.uri, glib::UriFlags::NONE)
                    .ok()?
                    .host()
                {
                    Some(host) => Some(ssh_host_lookup(&host).await),
                    _ => None,
                }
            }
        }
    }

    pub async fn host_address(&self) -> Option<gio::InetAddress> {
        if let Some(host) = self.host().await {
            gio::Resolver::default()
                .lookup_by_name_future(&host)
                .await
                .ok()
                .and_then(|x| x.first().cloned())
        } else {
            None
        }
    }

    pub async fn is_internet(&self) -> bool {
        match self.host_address().await {
            Some(host_address) => !host_address.is_site_local(),
            _ => false,
        }
    }

    pub fn icon(&self) -> String {
        match self {
            Self::Local(local) => local.icon.clone().unwrap_or_else(|| String::from("folder")),
            Self::Remote(_) => String::from("network-server"),
        }
    }

    pub fn icon_symbolic(&self) -> String {
        match self {
            Self::Local(local) => local
                .icon_symbolic
                .clone()
                .unwrap_or_else(|| String::from("folder-symbolic")),
            Self::Remote(_) => String::from("network-server-symbolic"),
        }
    }

    pub fn location(&self) -> String {
        if let Self::Local(local) = self {
            format!(
                "{} – {}",
                local.mount_name.as_deref().unwrap_or_default(),
                self.subtitle(),
            )
        } else {
            self.to_string()
        }
    }

    pub fn uri_fuse(&self) -> Option<String> {
        match self {
            Self::Local(local::Repository { uri: Some(uri), .. })
                if !gio::File::for_uri(uri).is_native() =>
            {
                Some(uri.clone())
            }
            _ => None,
        }
    }

    pub fn is_filesystem(&self) -> bool {
        matches!(self, Self::Local(_))
    }

    pub fn is_network(&self) -> bool {
        matches!(self, Self::Remote(_)) || self.uri_fuse().is_some()
    }

    pub fn is_drive_removable(&self) -> bool {
        match self {
            Self::Local(local::Repository { removable, .. }) => *removable,
            _ => false,
        }
    }

    pub fn is_drive_connected(&self) -> Option<bool> {
        if self.is_drive_removable() {
            Some(self.removable_drive_volume().is_some())
        } else {
            None
        }
    }

    pub fn is_drive_mounted(&self) -> Option<bool> {
        self.removable_drive_volume()
            .map(|volume| volume.get_mount().is_some())
    }

    pub fn is_drive_ejectable(&self) -> Option<bool> {
        self.removable_drive_volume()
            .and_then(|volume| volume.drive())
            .map(|drive| drive.can_eject())
    }

    pub fn removable_drive_volume(&self) -> Option<gio::Volume> {
        match self {
            Self::Local(local::Repository {
                removable,
                volume_uuid: Some(volume_uuid),
                ..
            }) if *removable => gio::VolumeMonitor::get().volume_for_uuid(volume_uuid),
            _ => None,
        }
    }

    /// Auto-generated title fallback
    pub fn title_fallback(&self) -> String {
        match self {
            Self::Local(local) => local.mount_name.clone().unwrap_or_default(),
            Self::Remote(_) => gettext("Remote Location"),
        }
    }

    pub fn subtitle(&self) -> String {
        match self {
            Self::Local(local) => local
                .drive_name
                .clone()
                .or_else(|| self.uri_fuse())
                .unwrap_or_else(|| self.to_string()),
            Self::Remote(_) => self.to_string(),
        }
    }

    pub fn set_settings(&mut self, settings: Option<BackupSettings>) {
        *match self {
            Self::Local(local) => &mut local.settings,
            Self::Remote(remote) => &mut remote.settings,
        } = settings;
    }

    pub fn settings(&self) -> Option<BackupSettings> {
        match self {
            Self::Local(local) => &local.settings,
            Self::Remote(remote) => &remote.settings,
        }
        .clone()
    }
}

impl std::fmt::Display for Repository {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let repo = match self {
            Self::Local(local) => local.path().to_string_lossy().to_string(),
            Self::Remote(remote) => remote.uri.to_string(),
        };
        write!(f, "{repo}")
    }
}
