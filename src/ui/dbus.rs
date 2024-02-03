use crate::ui::prelude::*;
use async_std::prelude::*;

use crate::{schedule, ui};
use async_std::channel::Sender;

struct PikaBackup {
    command: Sender<Command>,
}

#[derive(Debug)]
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
        } else {
            debug!("Command to start scheduled backup sent.")
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

pub async fn init() {
    let (sender, mut receiver) = async_std::channel::unbounded();

    Handler::run(async move {
        debug!("Internally awaiting D-Bus API commands");
        while let Some(command) = receiver.next().await {
            debug!("Received D-Bus API command {command:?}");
            match command {
                Command::StartBackup(config_id, due_cause) => {
                    ui::page_backup::dbus_start_backup(config_id, due_cause).await?
                }
                Command::ShowOverview => ui::page_overview::dbus_show(),
                Command::ShowSchedule(backup_id) => ui::page_schedule::dbus_show(backup_id),
            }
        }

        Ok(())
    });

    Handler::run(async move {
        spawn_server(sender)
            .await
            .err_to_msg(gettext("Failed to spawn interface for scheduled backups."))
    });
}

async fn spawn_server(command: Sender<Command>) -> zbus::Result<()> {
    let zbus_session = crate::utils::dbus::session().await?;
    zbus_session
        .object_server()
        .at(crate::DBUS_API_PATH, PikaBackup { command })
        .await?;

    zbus_session.request_name(crate::DBUS_API_NAME).await?;

    debug!("D-Bus listening on {}", crate::DBUS_API_NAME);

    Ok(())
}
