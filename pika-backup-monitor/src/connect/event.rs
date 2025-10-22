use gio::prelude::*;

use crate::action;
use crate::notification::Note;
use crate::prelude::*;

pub fn volume_added(volume: &gio::Volume) {
    let uuid = volume.uuid().unwrap_or_default();
    tracing::debug!("Volume added {:?}", uuid);

    let backups = BACKUP_CONFIG.load();
    let backups = backups
        .iter()
        .filter(|backup| match &backup.repo {
            common::config::Repository::Local(repo) => {
                repo.is_likely_on_volume(volume) && !backup.schedule.enabled
            }
            common::config::Repository::Remote(_) => false,
        })
        .collect::<Vec<_>>();

    if let Some(first_backup) = backups.first() {
        let notification = gio::Notification::new(&gettext("Backup Device Connected"));

        if backups.len() > 1 {
            tracing::debug!("Device has several configured backups without schedule");

            notification.set_body(Some(&gettextf(
                "“{}” contains multiple configured backups.",
                [&first_backup.repo.location()],
            )));

            notification.add_button_with_target_value(
                &gettext("Show Backups"),
                &action::ShowOverview::name(),
                None,
            );
        } else {
            tracing::debug!("Device has one configured backup without schedule");

            notification.set_body(Some(&gettextf(
                "“{}” contains one configured backup.",
                [&first_backup.repo.location()],
            )));

            notification.add_button_with_target_value(
                &gettext("Back Up Now"),
                &action::StartBackup::name(),
                Some(&first_backup.id.to_variant()),
            );
        }

        notification.set_default_action(&action::ShowOverview::name());
        gio_app().send_notification(
            Some(&Note::DeviceAvailable(&uuid).to_string()),
            &notification,
        );
    }
}

pub fn volume_removed(volume: &gio::Volume) {
    let uuid = volume.uuid();
    tracing::debug!("Volume removed {:?}", uuid);

    if let Some(uuid) = uuid {
        gio_app().withdraw_notification(&Note::DeviceAvailable(&uuid).to_string());
    }
}
