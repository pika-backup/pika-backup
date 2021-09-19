use crate::config;
use crate::daemon::globals::gio_app;
use crate::daemon::prelude::*;
use gio::prelude::*;

pub fn mount_added(mount: &gio::Mount) {
    let backups = &BACKUP_CONFIG.load();
    let uuid = crate::utils::mount_uuid(mount);
    debug!("Log: Connected {:?}", uuid);
    if let Some(uuid) = uuid {
        let backup = backups.iter().find(|b| {
            debug!("Log: Checking {:?}", &b);
            if let config::Backup {
                repo: config::Repository::Local(local),
                ..
            } = b
            {
                local.volume_uuid.as_ref() == Some(&uuid)
            } else {
                false
            }
        });

        if let Some(config::Backup {
            id,
            repo: config::Repository::Local(local),
            ..
        }) = backup
        {
            let notification = gio::Notification::new(&gettext("Backup Medium Connected"));
            notification.set_body(Some(
                gettextf(
                    "{} on Disk „{}“",
                    &[
                        local.mount_name.as_ref().unwrap(),
                        local.drive_name.as_ref().unwrap(),
                    ],
                )
                .as_str(),
            ));

            notification.add_button_with_target_value(
                &gettext("Run Backup"),
                &format!("app.{}", crate::action::backup_start().name()),
                Some(&id.to_string().to_variant()),
            );
            gio_app().send_notification(Some(uuid.as_str()), &notification);
            debug!("Log: Notification send");
        }
    }
}
