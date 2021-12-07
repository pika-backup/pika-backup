/*!
# Daemon initialization
*/

use super::requirements;
use crate::action;
use crate::config;
use crate::daemon::error::Result;
use crate::daemon::prelude::*;
use crate::daemon::utils;
use arc_swap::ArcSwap;
use futures::stream::StreamExt;
use gio::prelude::*;
use once_cell::sync::Lazy;
use std::sync::Arc;

thread_local!(
    static ACTION_GROUP: gio::DBusActionGroup = gio::DBusActionGroup::get(
        &gio_app().dbus_connection().unwrap(),
        Some(&crate::app_id()),
        &format!("/{}", crate::app_id().replace(".", "/")),
    );
);

pub static BACKUP_STATUS: Lazy<ArcSwap<Option<action::BackupStatus>>> = Lazy::new(Default::default);

async fn listen_remote_app_running() -> Result<()> {
    let conn = zbus::Connection::session().await?;
    let mut stream = zbus::fdo::DBusProxy::new(&conn)
        .await?
        .receive_name_owner_changed()
        .await?;

    while let Some(signal) = stream.next().await {
        let args = signal.args()?;
        if args.name == application_id!() && args.new_owner.is_none() {
            debug!("Remote app '{}' closed.", args.name);
            BACKUP_STATUS.store(Default::default());
        }
    }

    Ok(())
}

pub fn init() {
    glib::MainContext::default().spawn(async {
        listen_remote_app_running()
            .await
            .handle("Listening to remote app status failed.")
    });

    init_backup_status_monitor();
    super::status::load();

    glib::timeout_add_seconds(60, minutely);
}

fn init_backup_status_monitor() {
    ACTION_GROUP.with(|action_group| {
        action_group.connect_action_added(Some("backup.status"), |action_group, action_name| {
            let result: Option<action::BackupStatus> = action_group
                .action_state(action_name)
                .as_ref()
                .and_then(glib::FromVariant::from_variant);
            debug!("Backup status update: {:?}", result);
            BACKUP_STATUS.store(Arc::new(result));
        });

        action_group.connect_action_state_changed(Some("backup.status"), |_, _, value| {
            let result: Option<action::BackupStatus> = glib::FromVariant::from_variant(value);
            debug!("Backup status available: {:?}", result);
            BACKUP_STATUS.store(Arc::new(result));
        });

        // trigger update
        action_group.list_actions();
    });
}

fn minutely() -> glib::Continue {
    debug!("Probing schedules");

    for config in BACKUP_CONFIG.load().iter() {
        if config.schedule.enabled {
            probe(config);
        }
    }
    track_activity();

    glib::Continue(true)
}

fn track_activity() {
    for config in BACKUP_CONFIG.load().iter() {
        if config.schedule.enabled
            && !matches!(config.schedule.frequency, config::Frequency::Hourly)
        {
            SCHEDULE_STATUS.update_return(|s| {
                let activity = s.activity.entry(config.id.clone()).or_default();

                activity.tick()
            });
        }
    }

    super::status::write();
}

fn probe(config: &config::Backup) {
    let schedule = &config.schedule;
    debug!("---");
    debug!("Probing backup: {}", config.repo);
    debug!("Frequency: {:?}", schedule.frequency);

    let global = requirements::Global::check(config, BACKUP_HISTORY.load().as_ref());
    let due = requirements::Due::check(config, BACKUP_HISTORY.load().as_ref());

    if !global.is_empty() || due.is_err() {
        debug!("Some requirements are not met");
        debug!("Global requirement: {:?}", global);
        debug!("Due requirement: {:?}", due);
    } else {
        info!("Trying to start backup '{}'", config.id);
        utils::forward_action(
            &crate::action::backup_start(),
            Some(&config.id.to_variant()),
        );
    }
}
