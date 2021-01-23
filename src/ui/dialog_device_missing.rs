use gio::prelude::*;
use gtk::prelude::*;
use std::rc::Rc;

use crate::shared;
use crate::ui;
use crate::ui::globals::*;
use crate::ui::prelude::*;

pub fn main<F: Fn() + Send + 'static>(config: shared::BackupConfig, purpose: &str, f: F) {
    match &config.repo {
        shared::BackupRepo::Local {
            path, removable, ..
        } if !ui::utils::is_backup_repo(path) => {
            if let Some(uri) = config.repo.get_uri_fuse() {
                mount_fuse_dialog(uri, f);
            } else if *removable {
                await_mount_dialog(config, purpose, f);
            } else {
                f();
            }
        }
        _ => {
            f();
        }
    }
}

fn mount_added<F: Fn()>(
    repo_path: &std::path::Path,
    new_mount: &gio::Mount,
    dialog: Rc<ui::builder::DialogDeviceMissing>,
    f: &F,
) {
    debug!("Mount added");
    if repo_path.starts_with(new_mount.get_root().unwrap().get_path().unwrap()) {
        debug!("Looks like the correct mount");
        dialog.window().close();
        f();
    }
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
                Some(gio::IOErrorEnum::FailedHandled) => Err(UserAborted {}.into()),
                _ => Err(Message::new(gettext("Failed to mount."), err).into()),
            }
        }
    }
}

pub fn mount_fuse_dialog<F: Fn() + 'static>(uri: String, f: F) {
    let file = gio::File::new_for_uri(&uri);
    spawn_local(async move {
        if mount_enclosing(&file).await.is_ok() {
            f();
        }
    });
}

fn await_mount_dialog<F: Fn() + 'static>(config: shared::BackupConfig, purpose: &str, f: F) {
    if let shared::BackupRepo::Local {
        mount_name,
        drive_name,
        icon,
        path,
        ..
    } = config.repo
    {
        let dialog = Rc::new(ui::builder::DialogDeviceMissing::new());

        if !purpose.is_empty() {
            dialog.purpose().set_text(purpose);
            dialog.purpose().show();
        }

        let volume_monitor = gio::VolumeMonitor::get();

        volume_monitor.connect_mount_added(
            enclose!((dialog, path) move |_, new_mount| mount_added(&path, new_mount, dialog.clone(), &f)),
        );

        dialog.cancel().connect_clicked(enclose!((dialog) move |_| {
            dialog.window().close();
            // this line triggers a move of volume_monior
            volume_monitor.is::<bool>();
        }));

        dialog.window().set_transient_for(Some(&main_ui().window()));
        dialog.device().set_label(&drive_name.unwrap_or_default());
        dialog.mount().set_label(&mount_name.unwrap_or_default());
        if let Some(g_icon) = icon.and_then(|x| gio::Icon::new_for_string(&x).ok()) {
            let img = gtk::Image::from_gicon(&g_icon, gtk::IconSize::Dialog);
            img.set_pixel_size(128);
            dialog.icon().add(&img);
        }
        dialog.window().show_all();
    }
}
