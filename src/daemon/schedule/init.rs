/*!
# Daemon initialization
*/
use crate::daemon::prelude::*;
use gio::prelude::*;
use std::collections::HashMap;

use crate::config;
use crate::daemon::{action, dbus, notification::Note, schedule};
use crate::schedule::requirements;

pub fn init() {
    super::status::load();

    glib::timeout_add_seconds(schedule::PROBE_FREQUENCY.as_secs() as u32, minutely);
}

fn minutely() -> glib::ControlFlow {
    debug!("Probing schedules");

    for config in BACKUP_CONFIG.load().iter() {
        if config.schedule.enabled {
            glib::MainContext::default().block_on(probe(config));
        }
    }
    track_activity();

    glib::ControlFlow::Continue
}

fn track_activity() {
    for config in BACKUP_CONFIG.load().iter() {
        if config.schedule.enabled
            && !matches!(config.schedule.frequency, config::Frequency::Hourly)
        {
            SCHEDULE_STATUS.update(|s| {
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

pub struct Reminder;

impl Reminder {
    fn is_remind_again(id: &ConfigId) -> bool {
        !matches!(LAST_REMINDED.load().get(id), Some(instant) if instant.elapsed() < super::REMIND_UNMET_CRITERIA)
    }

    fn reminded_now(id: &ConfigId) {
        LAST_REMINDED.rcu(|x| {
            let mut new = HashMap::clone(x);
            new.insert(id.clone(), std::time::Instant::now());
            new
        });
    }
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
            if let Some(global_first) = global.first() {
                debug!("Global requirements are not met: {:#?}", global);
                if Reminder::is_remind_again(&config.id) {
                    let body = match global_first {
                        requirements::Global::ThisBackupRunning => None,
                        requirements::Global::OtherBackupRunning(_) => {
                            Some(gettext("The backup repository is already in use."))
                        }
                        requirements::Global::MeteredConnection => {
                            Some(gettext("Only metered internet connections available."))
                        }
                        requirements::Global::OnBattery => {
                            Some(gettext("Device not connected to power."))
                        }
                    };

                    if body.is_some() {
                        let notification =
                            gio::Notification::new(&gettext("Scheduled Backup Postponed"));
                        notification.set_body(body.as_deref());
                        notification.set_default_action_and_target_value(
                            &action::ShowSchedule::name(),
                            Some(&config.id.to_variant()),
                        );

                        gio_app().send_notification(
                            Some(&Note::Postponed(&config.id).to_string()),
                            &notification,
                        );
                        Reminder::reminded_now(&config.id);
                    }
                }
            } else {
                let hint = requirements::Hint::check(config);

                if hint.contains(&requirements::Hint::DeviceMissing) {
                    // TODO: check if path maybe still exists despite device being undetected
                    debug!("Backup device is not connected");

                    if Reminder::is_remind_again(&config.id) {
                        debug!("Send reminding notification");
                        let notification =
                            gio::Notification::new(&gettext("Backup Device Required"));
                        notification.set_body(Some(&gettextf(
                            "“{}” has to be connected for the scheduled backup to start.",
                            &[&config.repo.location()],
                        )));
                        gio_app().send_notification(
                            Some(&Note::DeviceRequired(&config.id).to_string()),
                            &notification,
                        );
                        Reminder::reminded_now(&config.id);
                    }
                } else {
                    info!("Trying to start backup {:?}", config.id);
                    dbus::PikaBackup::start_scheduled_backup(&config.id, due_cause)
                        .await
                        .handle(gettext("Failed to start scheduled backup"));

                    // withdraw notifications
                    gio_app().withdraw_notification(&Note::Postponed(&config.id).to_string());
                    gio_app().withdraw_notification(&Note::DeviceRequired(&config.id).to_string());

                    // reset reminder if criteria are met to alert if they are violated again
                    Reminder::reminded_now(&config.id);
                }
            }
        }
        Err(err) => {
            debug!("Backup is not yet due: {:?}", err);
        }
    }
}
