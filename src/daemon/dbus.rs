use crate::daemon::prelude::*;
use crate::schedule;

use once_cell::sync::OnceCell;
use zbus::Result;

#[zbus::dbus_proxy(interface = "org.gnome.World.PikaBackup1", assume_defaults = false)]
trait PikaBackup {
    fn start_scheduled_backup(
        &self,
        config_id: &ConfigId,
        due_cause: schedule::DueCause,
    ) -> Result<()>;

    fn start_backup(&self, config_id: &ConfigId) -> Result<()>;

    fn show_overview(&self) -> Result<()>;

    fn show_schedule(&self, config_id: &ConfigId) -> Result<()>;
}

pub struct PikaBackup;

impl PikaBackup {
    pub async fn proxy() -> Result<PikaBackupProxy<'static>> {
        static PROXY: once_cell::sync::OnceCell<PikaBackupProxy<'static>> = OnceCell::new();

        if let Some(proxy) = PROXY.get() {
            Ok(proxy.clone())
        } else {
            let proxy = PikaBackupProxy::builder(&crate::utils::dbus::system().await?)
                .destination(crate::DBUS_API_NAME)?
                .path(crate::DBUS_API_PATH)?
                .build()
                .await?;
            Ok(PROXY.get_or_init(move || proxy).clone())
        }
    }

    pub async fn start_scheduled_backup(
        config_id: &ConfigId,
        due_cause: schedule::DueCause,
    ) -> Result<()> {
        Self::proxy()
            .await?
            .start_scheduled_backup(config_id, due_cause)
            .await
    }

    pub async fn start_backup(config_id: &ConfigId) -> Result<()> {
        Self::proxy().await?.start_backup(config_id).await
    }

    pub async fn show_overview() -> Result<()> {
        Self::proxy().await?.show_overview().await
    }

    pub async fn show_schedule(config_id: &ConfigId) -> Result<()> {
        Self::proxy().await?.show_schedule(config_id).await
    }
}
