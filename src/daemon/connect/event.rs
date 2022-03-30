use crate::daemon::prelude::*;
use gio::prelude::*;

use crate::config;
use crate::daemon::action;

pub fn mount_added(mount: &gio::Mount) {
    let backups = BACKUP_CONFIG.load();
    let uuid = crate::utils::mount_uuid(mount);
    debug!("Mount added {:?}, {:?}", uuid, mount.root().uri());

    if let Some(uuid) = uuid {
        let backups = backups
            .iter()
            .filter_map(|config| match &config.repo {
                repo @ config::Repository::Local(local)
                    if (local.volume_uuid.as_ref() == Some(&uuid)
                        || local.uri.as_deref() == Some(mount.root().uri().as_str()))
                        && !config.schedule.enabled =>
                {
                    eprintln!("match");
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

            gio_app().send_notification(Some(uuid.as_str()), &notification);
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

            gio_app().send_notification(Some(uuid.as_str()), &notification);
        }
    }
}
