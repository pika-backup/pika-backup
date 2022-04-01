use crate::daemon::prelude::*;
use gio::prelude::*;

use crate::config;
use crate::daemon::{action, notification::Note};

pub fn volume_added(volume: &gio::Volume) {
    let uuid = volume.uuid();
    debug!("Volume added {:?}", uuid);

    if let Some(uuid) = uuid {
        let backups = BACKUP_CONFIG.load();
        let backups = backups
            .iter()
            .filter_map(|config| match &config.repo {
                repo @ config::Repository::Local(local)
                    if local.volume_uuid.as_deref() == Some(uuid.as_str())
                        && !config.schedule.enabled =>
                {
                    Some((&config.id, local, repo))
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        if backups.len() > 1 {
            debug!("Device has several configured backups without schedule");

            let (_, _, repo) = backups[0];
            let notification = gio::Notification::new(&gettext("Backup Device Connected"));

            notification.set_body(Some(&gettextf(
                "“{}” contains multiple configured backups.",
                &[&repo.location()],
            )));

            notification.set_default_action(&action::ShowOverview::name());

            notification.add_button_with_target_value(
                &gettext("Show Backups"),
                &action::ShowOverview::name(),
                None,
            );

            gio_app().send_notification(
                Some(&Note::DeviceAvailable(&uuid).to_string()),
                &notification,
            );
        } else if let Some((id, _, repo)) = backups.first() {
            debug!("Device has one configured backup without schedule");

            let notification = gio::Notification::new(&gettext("Backup Device Connected"));

            notification.set_body(Some(&gettextf(
                "“{}” contains one configured backup.",
                &[&repo.location()],
            )));

            notification.set_default_action(&action::ShowOverview::name());

            notification.add_button_with_target_value(
                &gettext("Back Up Now"),
                &action::StartBackup::name(),
                Some(&id.to_variant()),
            );

            gio_app().send_notification(
                Some(&Note::DeviceAvailable(&uuid).to_string()),
                &notification,
            );
        }
    }
}

pub fn volume_removed(volume: &gio::Volume) {
    let uuid = volume.uuid();
    debug!("Volume removed {:?}", uuid);

    if let Some(uuid) = uuid {
        gio_app().withdraw_notification(&Note::DeviceAvailable(&uuid).to_string());
    }
}
