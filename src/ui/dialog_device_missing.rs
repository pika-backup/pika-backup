use async_std::prelude::*;
use gio::prelude::*;
use gtk::prelude::*;
use std::rc::Rc;

use crate::config;
use crate::ui;
use crate::ui::prelude::*;

pub fn set_mount_path(config: &mut config::Backup, mount: &gio::Mount) {
    if let Some(new_mount_path) = mount.root().path() {
        match config.repo {
            config::Repository::Local(ref mut repo @ config::local::Repository { .. }) => {
                if repo.mount_path != new_mount_path {
                    warn!(
                        "Repository mount path seems to have changed. Trying with this: {:?}",
                        new_mount_path
                    );

                    repo.mount_path = new_mount_path;
                } else {
                    debug!("Mount path still the same");
                }
            }
            _ => unreachable!(),
        }
    }
}

pub async fn updated_config(config: &config::Backup, purpose: &str) -> Result<config::Backup> {
    let mut new_config = config.clone();

    match &config.repo {
        config::Repository::Local(repo) => {
            if !ui::utils::is_backup_repo(&repo.path()) {
                if let Some(uri) = config.repo.uri_fuse() {
                    info!("Remote gvfs repo not available");
                    mount_enclosing(&gio::File::for_uri(&uri)).await?;
                } else if repo.removable {
                    info!("Removable drive not available");

                    // try to find volume with same uuid
                    let volume = gio::VolumeMonitor::get()
                        .volumes()
                        .into_iter()
                        .find(|v| repo.is_likely_on_volume(v));

                    if let Some(mount) = volume.as_ref().and_then(|v| v.get_mount()) {
                        info!("Probably found repo somewhere else");
                        set_mount_path(&mut new_config, &mount);
                    } else if let Some(new_volume) = volume {
                        error!("Not mounted yet. Mounting");
                        new_volume
                            .mount_future(
                                gio::MountMountFlags::NONE,
                                Some(&gtk::MountOperation::new(Some(&main_ui().window()))),
                            )
                            .await
                            .err_to_msg(gettext("Failed to Mount"))?;
                        if let Some(mount) = new_volume.get_mount() {
                            info!("Successfully mounted");
                            set_mount_path(&mut new_config, &mount);
                        } else {
                            return Err(Message::short(gettext("Failed to Mount")).into());
                        }
                    } else {
                        info!("Waiting for mount to appear");
                        let mount = mount_dialog(repo.clone(), purpose).await?;
                        set_mount_path(&mut new_config, &mount);
                    }
                } else {
                    info!("Local drive not available");
                }
            }
        }
        config::Repository::Remote { .. } => {
            // remote
        }
    }

    Ok(new_config)
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

    let uri_scheme = file
        .uri_scheme()
        .map(|scheme| scheme.to_ascii_lowercase())
        .unwrap_or_default();

    match mount_result.await {
        Ok(()) => Ok(()),
        Err(err) => match err.kind::<gio::IOErrorEnum>() {
            Some(gio::IOErrorEnum::AlreadyMounted) => {
                warn!("Tried to mount {file:#?} but it was already mounted");
                Ok(())
            }
            Some(gio::IOErrorEnum::FailedHandled) => Err(Error::UserCanceled),
            Some(gio::IOErrorEnum::Failed) | Some(gio::IOErrorEnum::InvalidArgument)
                if uri_scheme == "smb" =>
            {
                // SMB can give "Invalid Argument" or even just "Failed" as an error when
                // the network is unreachable.
                // See [gvfs issue #315](https://gitlab.gnome.org/GNOME/gvfs/-/issues/315)
                // and [DejaDup issue #406](https://gitlab.gnome.org/World/deja-dup/-/issues/406)

                Err(Message::new(
                    gettext("Failed to Mount"),
                    gettext("The network server is not available"),
                )
                .into())
            }
            _ => Err(Message::new(gettext("Failed to Mount"), err).into()),
        },
    }
}

async fn mount_dialog(repo: config::local::Repository, purpose: &str) -> Result<gio::Mount> {
    let dialog = Rc::new(ui::builder::DialogDeviceMissing::new());
    dialog.window().set_transient_for(Some(&main_ui().window()));
    dialog.window().set_title(Some(purpose));
    dialog
        .name()
        .set_label(&repo.clone().into_config().location());

    if let Some(g_icon) = repo
        .icon
        .as_ref()
        .and_then(|x| gio::Icon::for_string(x).ok())
    {
        let img = gtk::Image::from_gicon(&g_icon);
        img.set_pixel_size(128);
        dialog.icon().append(&img);
    }

    let volume_monitor = gio::VolumeMonitor::get();
    let (mount_sender, mut mount_receiver) = async_std::channel::unbounded();

    volume_monitor.connect_mount_added(enclose!((dialog, mount_sender) move |_, new_mount| {
        if let Some(volume) = new_mount.volume() {
            if repo.is_likely_on_volume(&volume) {
                let _ignore = mount_sender.try_send(Some(new_mount.clone()));
                dialog.window().close();
            } else {
                debug!("New volume, but likely not on there.");
            }
        }
    }));

    dialog.window().connect_close_request(move |_| {
        let _ignore = mount_sender.try_send(None);
        gtk::Inhibit(false)
    });

    dialog.window().show();

    mount_receiver
        .next()
        .await
        .flatten()
        .ok_or(Error::UserCanceled)
}
