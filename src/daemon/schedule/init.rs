/*!
# Daemon initialization
*/
use crate::daemon::prelude::*;

use crate::config;
use crate::daemon::dbus;
use crate::daemon::schedule;
use crate::schedule::requirements;

pub fn init() {
    super::status::load();

    glib::timeout_add_seconds(schedule::PROBE_FREQUENCY.as_secs() as u32, minutely);
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
                match BACKUP_HISTORY
                    .load()
                    .get_result(&config.id)
                    .ok()
                    .and_then(|x| x.last_completed.as_ref())
                {
                    Some(last_completed) if activity.last_update < last_completed.end => {
                        activity.reset()
                    }
                    _ => activity.tick(schedule::PROBE_FREQUENCY),
                }
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

    let due = requirements::Due::check(config);

    match due {
        Ok(due_cause) => {
            debug!("Backup is due because: {:?}", due_cause);
            let global = requirements::Global::check(config, BACKUP_HISTORY.load().as_ref()).await;
            if global.is_empty() {
                let hint = requirements::Hint::check(config);

                if hint.contains(&requirements::Hint::DeviceMissing) {
                    // TODO: check if path maybe still exists
                    debug!("Backup device is not connected");
                } else {
                    info!("Trying to start backup {:?}", config.id);
                    dbus::PikaBackup::start_scheduled_backup(&config.id, due_cause)
                        .await
                        .handle(gettext("Failed to start scheduled backup"));
                }
            } else {
                debug!("Global requirements are not met: {:#?}", global);
            }
        }
        Err(err) => {
            debug!("Backup is not yet due: {:?}", err);
        }
    }
}
