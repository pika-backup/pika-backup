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
use chrono::prelude::*;
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
    let conn = zbus::azync::Connection::session().await?;
    let mut stream = zbus::fdo::AsyncDBusProxy::new(&conn)
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
    super::status::load();

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
    for config in BACKUP_CONFIG.load().iter() {
        if config.schedule.enabled {
            probe(config);
        }
    }
    track_activity();

    glib::Continue(true)
}

fn track_activity() {
    let mut changed = false;

    for config in BACKUP_CONFIG.load().iter() {
        if config.schedule.enabled
            && !matches!(config.schedule.frequency, config::Frequency::Hourly)
        {
            changed |= SCHEDULE_STATUS.update_return(|s| {
                if let Some(activity) = s.activity.get_mut(&config.id) {
                    activity.tick()
                } else {
                    s.activity.insert(
                        config.id.clone(),
                        config::Activity {
                            last_activity: chrono::Local::now(),
                            minutes_past: 0,
                            minutes_today: 1,
                        },
                    );
                    true
                }
            });
        }
    }

    if changed {
        debug!("Schedule status acticity changed.");
        super::status::write();
    }
}

fn probe(config: &config::Backup) {
    let time0 = chrono::Local.timestamp(0, 0);

    let schedule = &config.schedule;
    debug!("---");
    debug!("Probing backup: {}", config.repo);
    debug!("Frequency: {:?}", schedule.frequency);

    if let Err(reason) = requirements::Global::check(config) {
        debug!("Global requirement not fulfilled: {:?}", reason);
        return;
    }

    let history_all = BACKUP_HISTORY.load();
    let history = history_all.get_result(&config.id).ok();

    let last_run = history.and_then(|x| x.run.front());
    let last_completed = history.and_then(|x| x.last_completed.as_ref());

    let last_run_datetime = last_run.map(|x| x.end).unwrap_or(time0);

    debug!("Last run: {:?}", last_run_datetime);
    debug!("Last completed: {:?}", last_completed.map(|x| x.end));

    let last_run_ago = chrono::Local::now() - last_run.map_or(time0, |x| x.end);

    match schedule.frequency {
        config::Frequency::Hourly => {
            if last_run_ago >= chrono::Duration::hours(1) {
                info!("Trying to start backup '{}'", config.id);
                utils::forward_action(
                    &crate::action::backup_start(),
                    Some(&config.id.to_variant()),
                );
            } else {
                debug!(
                    "Last backup is only {} minutes ago.",
                    last_run_ago.num_minutes()
                );
            }
        }
        config::Frequency::Daily { preferred_time } => {
            let scheduled_datetime = chrono::Local::today().and_time(preferred_time).unwrap();
            if last_run_datetime < scheduled_datetime && scheduled_datetime < chrono::Local::now() {
                info!("Trying to start backup '{}'", config.id);
                utils::forward_action(
                    &crate::action::backup_start(),
                    Some(&config.id.to_variant()),
                );
            }
        }
        _ => error!("Not supported yet."),
    }
}
