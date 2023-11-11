use crate::ui::prelude::*;
use async_std::prelude::*;

use crate::schedule::requirements;
use crate::{schedule, ui};
use async_std::channel::Sender;

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

        // As this is a scheduled backup we will verify the schedule ourselves.
        // This mitigates an issue where an outdated daemon binary with schedule bugs could cause backups every minute.
        match BACKUP_CONFIG
            .load()
            .try_get(&config_id)
            .map(requirements::Due::check)
        {
            Ok(Ok(cause)) => {
                if cause != due_cause {
                    warn!("The monitor process asked us to start a scheduled backup '{config_id}' but we disagree on the reason. Starting anyway: {cause:?} != {due_cause:?}");
                }

                if let Err(err) = self
                    .command
                    .send(Command::StartBackup(config_id, Some(due_cause)))
                    .await
                {
                    error!("{}", err);
                }
            }
            Ok(Err(due)) => {
                match due {
                    requirements::Due::NotDue { next } => warn!(
                        "The monitor process asked us to start a scheduled backup '{config_id}' but it's not due yet. Next backup: {next:?}"
                    ),
                    requirements::Due::Running => warn!(
                        "The monitor process asked us to start a scheduled backup '{config_id}' but it's already running"
                    ),
                }
            }
            Err(err) => {
                error!("The monitor process asked us to start a scheduled backup with unknown config id: {err:?}");
            }
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
    let zbus_session = crate::utils::dbus::session().await?;
    zbus_session
        .object_server()
        .at(crate::DBUS_API_PATH, PikaBackup { command })
        .await?;

    zbus_session.request_name(crate::DBUS_API_NAME).await
}
