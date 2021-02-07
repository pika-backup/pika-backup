use gio::prelude::*;
use gtk::prelude::*;
use std::rc::Rc;

use crate::config;
use crate::ui;
use crate::ui::globals::*;
use crate::ui::prelude::*;

pub async fn updated_config(
    config: config::BackupConfig,
    purpose: &str,
) -> Result<config::BackupConfig> {
    match &config.repo {
        config::BackupRepo::Local {
            path, removable, ..
        } => {
            if !ui::utils::is_backup_repo(path) {
                if let Some(uri) = config.repo.get_uri_fuse() {
                    mount_enclosing(&gio::File::new_for_uri(&uri)).await?;
                } else if *removable {
                    mount_dialog(config.clone(), purpose).await?;
                }
            }
        }
        config::BackupRepo::Remote { .. } => {
            // remote
        }
    }

    Ok(config)
}

pub async fn mount_enclosing(file: &gio::File) -> Result<()> {
    info!("Trying to mount '{}'", file.get_uri());
    let mount_result = file.mount_enclosing_volume_future(
        gio::MountMountFlags::NONE,
        Some(
            &gtk::MountOperationBuilder::new()
                .parent(&main_ui().window())
                .build(),
        ),
    );

    match mount_result.await {
        Ok(()) => Ok(()),
        Err(err) => {
            match err.kind::<gio::IOErrorEnum>() {
                // TODO
                Some(gio::IOErrorEnum::FailedHandled) => Err(UserAborted::new().into()),
                _ => Err(Message::new(gettext("Failed to mount."), err).into()),
            }
        }
    }
}

async fn mount_dialog(config: config::BackupConfig, purpose: &str) -> Result<()> {
    if let config::BackupRepo::Local {
        mount_name,
        drive_name,
        icon,
        path,
        ..
    } = config.repo
    {
        let dialog = Rc::new(ui::builder::DialogDeviceMissing::new());
        dialog.window().set_transient_for(Some(&main_ui().window()));
        dialog.device().set_label(&drive_name.unwrap_or_default());
        dialog.mount().set_label(&mount_name.unwrap_or_default());
        dialog.purpose().set_text(purpose);

        if let Some(g_icon) = icon.and_then(|x| gio::Icon::new_for_string(&x).ok()) {
            let img = gtk::Image::from_gicon(&g_icon, gtk::IconSize::Dialog);
            img.set_pixel_size(128);
            dialog.icon().add(&img);
        }

        dialog.cancel().connect_clicked(enclose!((dialog) move |_| {
            dialog.window().response(gtk::ResponseType::Cancel);
        }));

        let volume_monitor = gio::VolumeMonitor::get();

        volume_monitor.connect_mount_added(enclose!((dialog, path) move |_, new_mount| {
            if path.starts_with(new_mount.get_root().unwrap().get_path().unwrap()) {
                dialog.window().response(gtk::ResponseType::Ok);
            }
        }));

        let response = dialog.window().run_future().await;

        dialog.window().close();

        if response == gtk::ResponseType::Ok {
            Ok(())
        } else {
            Err(UserAborted::new().into())
        }
    } else {
        // TODO this cannot happen
        Err(UserAborted::new().into())
    }
}
