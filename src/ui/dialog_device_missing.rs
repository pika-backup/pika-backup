use gio::prelude::*;
use gtk::prelude::*;
use std::rc::Rc;

use crate::config;
use crate::ui;
use crate::ui::prelude::*;

pub async fn updated_config(config: config::Backup, purpose: &str) -> Result<config::Backup> {
    match &config.repo {
        config::Repository::Local(repo) => {
            if !ui::utils::is_backup_repo(&repo.path()) {
                if let Some(uri) = config.repo.uri_fuse() {
                    mount_enclosing(&gio::File::for_uri(&uri)).await?;
                } else if repo.removable {
                    mount_dialog(repo.clone(), purpose).await?;
                }
            }
        }
        config::Repository::Remote { .. } => {
            // remote
        }
    }

    Ok(config)
}

pub async fn mount_enclosing(file: &gio::File) -> Result<()> {
    info!("Trying to mount '{}'", file.uri());
    let mount_result = file.mount_enclosing_volume_future(
        gio::MountMountFlags::NONE,
        Some(
            &gtk::MountOperation::builder()
                .parent(&main_ui().window())
                .build(),
        ),
    );

    match mount_result.await {
        Ok(()) => Ok(()),
        Err(err) => {
            match err.kind::<gio::IOErrorEnum>() {
                // TODO
                Some(gio::IOErrorEnum::FailedHandled) => Err(UserCanceled::new().into()),
                _ => Err(Message::new(gettext("Failed to mount."), err).into()),
            }
        }
    }
}

async fn mount_dialog(repo: config::local::Repository, purpose: &str) -> Result<()> {
    let path = repo.path();

    let dialog = Rc::new(ui::builder::DialogDeviceMissing::new());
    dialog.window().set_transient_for(Some(&main_ui().window()));
    dialog.purpose().set_text(purpose);
    dialog
        .name()
        .set_label(&repo.clone().into_config().location());

    if let Some(g_icon) = repo.icon.and_then(|x| gio::Icon::for_string(&x).ok()) {
        let img = gtk::Image::from_gicon(&g_icon, gtk::IconSize::Dialog);
        img.set_pixel_size(128);
        dialog.icon().add(&img);
        dialog.icon().show_all();
    }

    dialog.cancel().connect_clicked(enclose!((dialog) move |_| {
        dialog.window().response(gtk::ResponseType::Cancel);
    }));

    let volume_monitor = gio::VolumeMonitor::get();

    volume_monitor.connect_mount_added(enclose!((dialog) move |_, new_mount| {
        if path.starts_with(new_mount.root().path().unwrap()) {
            dialog.window().response(gtk::ResponseType::Ok);
        }
    }));

    let response = dialog.window().run_future().await;

    dialog.window().close();

    if response == gtk::ResponseType::Ok {
        Ok(())
    } else {
        Err(UserCanceled::new().into())
    }
}
