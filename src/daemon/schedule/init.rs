/*!
# Daemon initialization
*/

use super::requirements;

use crate::config;

use crate::daemon::prelude::*;
use crate::daemon::utils;

use gio::prelude::*;

thread_local!(
    static ACTION_GROUP: gio::DBusActionGroup = gio::DBusActionGroup::get(
        &gio_app().dbus_connection().unwrap(),
        Some(&crate::app_id()),
        &format!("/{}", crate::app_id().replace('.', "/")),
    );
);

pub fn init() {
    super::status::load();

    glib::timeout_add_seconds(60, minutely);
}

fn minutely() -> glib::Continue {
    debug!("Probing schedules");

    for config in BACKUP_CONFIG.load().iter() {
        if config.schedule.enabled {
            glib::MainContext::default().block_on(probe(config));
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

async fn probe(config: &config::Backup) {
    let schedule = &config.schedule;
    debug!("---");
    debug!("Probing backup: {}", config.repo);
    debug!("Frequency: {:?}", schedule.frequency);

    let global = requirements::Global::check(config, BACKUP_HISTORY.load().as_ref()).await;
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
