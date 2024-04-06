use crate::daemon::prelude::*;
use crate::schedule;

use zbus::Result;

/// Session Bus
pub async fn session_connection() -> Result<zbus::Connection> {
    static CONNECTION: async_lock::Mutex<Option<zbus::Connection>> = async_lock::Mutex::new(None);

    let mut connection = CONNECTION.lock().await;

    if let Some(connection) = &*connection {
        Ok(connection.clone())
    } else {
        let new_connection = zbus::Connection::session().await?;
        *connection = Some(new_connection.clone());
        Ok(new_connection)
    }
}

#[zbus::proxy(interface = "org.gnome.World.PikaBackup1", assume_defaults = false)]
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
        static PROXY: async_lock::Mutex<Option<PikaBackupProxy<'static>>> =
            async_lock::Mutex::new(None);

        let mut proxy = PROXY.lock().await;

        if let Some(proxy) = &*proxy {
            Ok(proxy.clone())
        } else {
            let new_proxy = PikaBackupProxy::builder(&session_connection().await?)
                .destination(crate::DBUS_API_NAME)?
                .path(crate::DBUS_API_PATH)?
                .build()
                .await?;
            *proxy = Some(new_proxy.clone());
            Ok(new_proxy)
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
