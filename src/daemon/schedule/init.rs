/*!
# Daemon initialization
*/
use crate::daemon::prelude::*;

use crate::config;
use crate::daemon::dbus;
use crate::schedule::requirements;

pub fn init() {
    super::status::load();

    glib::timeout_add_seconds(
        crate::daemon::schedule::SCHEDULE_PROBE_FREQUENCY.as_secs() as u32,
        minutely,
    );
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
        info!("Trying to start backup {:?}", config.id);
        dbus::PikaBackup::start_scheduled_backup(&config.id)
            .await
            .handle(gettext("Failed to start scheduled backup"));
    }
}
