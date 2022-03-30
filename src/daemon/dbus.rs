use crate::daemon::prelude::*;

use zbus::Result;

use crate::schedule;

#[zbus::dbus_proxy(interface = "org.gnome.World.PikaBackup1")]
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
        PikaBackupProxy::builder(&ZBUS_SESSION)
            .destination(crate::dbus_api_name())?
            .path(crate::dbus_api_path())?
            .build()
            .await
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
