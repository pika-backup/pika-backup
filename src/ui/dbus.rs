use crate::ui::prelude::*;
use async_std::prelude::*;

use crate::{schedule, ui};
use async_std::channel::Sender;
use once_cell::sync::Lazy;

struct PikaBackup {
    command: Sender<Command>,
}

enum Command {
    StartBackup(ConfigId, Option<schedule::DueCause>),
    ShowOverview,
    ShowSchedule(ConfigId),
}

#[zbus::dbus_interface(name = "org.gnome.World.PikaBackup1")]
impl PikaBackup {
    async fn start_scheduled_backup(&self, config_id: ConfigId, due_cause: schedule::DueCause) {
        info!(
            "Request to start scheduled backup {:?} {:?}",
            config_id, due_cause
        );
        if let Err(err) = self
            .command
            .send(Command::StartBackup(config_id, Some(due_cause)))
            .await
        {
            error!("{}", err);
        }
    }

    async fn start_backup(&self, config_id: ConfigId) {
        info!("Request to start backup {:?}", config_id);
        if let Err(err) = self
            .command
            .send(Command::StartBackup(config_id, None))
            .await
        {
            error!("{}", err);
        }
    }

    async fn show_overview(&self) {
        info!("Request to show overview");
        if let Err(err) = self.command.send(Command::ShowOverview).await {
            error!("{}", err);
        }
    }

    async fn show_schedule(&self, config_id: ConfigId) {
        info!("Request to show schedule {:?}", config_id);
        if let Err(err) = self.command.send(Command::ShowSchedule(config_id)).await {
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
        while let Some(command) = receiver.next().await {
            match command {
                Command::StartBackup(config_id, due_cause) => {
                    ui::page_backup::dbus_start_backup(config_id, due_cause)
                }
                Command::ShowOverview => ui::page_overview::dbus_show(),
                Command::ShowSchedule(backup_id) => ui::page_schedule::dbus_show(backup_id),
            }
        }
        Ok(())
    })
}

async fn spawn_server(command: Sender<Command>) -> zbus::Result<()> {
    ZBUS_SESSION
        .object_server()
        .at(crate::dbus_api_path(), PikaBackup { command })
        .await?;

    ZBUS_SESSION.request_name(crate::dbus_api_name()).await
}
