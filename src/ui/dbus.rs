use crate::ui::prelude::*;
use async_std::prelude::*;

use crate::ui;
use async_std::channel::Sender;
use once_cell::sync::Lazy;

struct PikaBackup {
    sender: Sender<ConfigId>,
}

#[zbus::dbus_interface(name = "org.gnome.World.PikaBackup1")]
impl PikaBackup {
    async fn start_scheduled_backup(&self, config_id: ConfigId) {
        info!("Request to start scheduled backup {:?}", config_id);
        if let Err(err) = self.sender.send(config_id).await {
            error!("{}", err);
        }
    }
}

pub fn init() {
    Lazy::force(&ZBUS_SESSION);

    let (sender, mut receiver) = async_std::channel::unbounded();

    Handler::run(async move {
        spawn_server(sender)
            .await
            .err_to_msg(gettext("Failed to spawn interface for scheduled backups."))
    });

    Handler::run(async move {
        while let Some(config_id) = receiver.next().await {
            ui::page_backup::activate_action_backup(config_id);
        }
        Ok(())
    })
}

async fn spawn_server(sender: Sender<ConfigId>) -> zbus::Result<()> {
    ZBUS_SESSION
        .object_server()
        .at(crate::dbus_api_path(), PikaBackup { sender })
        .await?;

    ZBUS_SESSION.request_name(crate::dbus_api_name()).await
}
