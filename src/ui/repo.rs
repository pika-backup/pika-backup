use gio::prelude::*;

use crate::ui::prelude::*;
use crate::ui::widget::DeviceMissingDialog;
use crate::{config, ui};

// Try to find volume that contains the repository
fn find_volume(repo: &config::local::Repository) -> Option<gio::Volume> {
    gio::VolumeMonitor::get()
        .volumes()
        .into_iter()
        .find(|v| repo.is_likely_on_volume(v))
}

/// Make sure the device is plugged in and available
///
/// No-Op for remote archives
pub async fn ensure_device_plugged_in(
    parent: &impl IsA<gtk::Widget>,
    config: &config::Backup,
    purpose: &str,
) -> Result<()> {
    if let config::Repository::Local(repo) = &config.repo
        && repo.removable
        && find_volume(repo).is_none()
    {
        let dialog = DeviceMissingDialog::new(config);
        dialog.present_with_repo(parent, repo, purpose).await?;
    }

    Ok(())
}

/// Check the current repository availability
///
/// If the repository is not available we try to mount it, showing the dialog if
/// required.
pub async fn ensure_repo_available(
    parent: &impl IsA<gtk::Widget>,
    config: &config::Backup,
    purpose: &str,
) -> Result<config::Backup> {
    let mut new_config = config.clone();

    match &config.repo {
        config::Repository::Local(repo) => {
            if !ui::utils::is_backup_repo(&repo.path()).await {
                if let Some(uri) = config.repo.uri_fuse() {
                    info!("Remote gvfs repo not available");
                    mount_enclosing(&gio::File::for_uri(&uri)).await?;
                } else if repo.removable {
                    info!("Removable drive not available");

                    // try to find volume with same uuid
                    let volume = find_volume(repo);

                    match volume.as_ref().and_then(|v| v.get_mount()) {
                        Some(mount) => {
                            info!("Probably found repo somewhere else");
                            new_config.set_mount_path(&mount);
                        }
                        _ => {
                            if let Some(new_volume) = volume {
                                error!("Not mounted yet. Mounting");
                                new_volume
                                    .mount_future(
                                        gio::MountMountFlags::NONE,
                                        Some(&gtk::MountOperation::new(Some(&main_ui().window()))),
                                    )
                                    .await
                                    .err_to_msg(gettext("Failed to Mount"))?;
                                match new_volume.get_mount() {
                                    Some(mount) => {
                                        info!("Successfully mounted");
                                        new_config.set_mount_path(&mount);
                                    }
                                    _ => {
                                        return Err(
                                            Message::short(gettext("Failed to Mount")).into()
                                        );
                                    }
                                }
                            } else {
                                info!("Waiting for mount to appear");
                                let dialog = DeviceMissingDialog::new(config);
                                let mount = dialog.present_with_repo(parent, repo, purpose).await?;
                                new_config.set_mount_path(&mount);
                            }
                        }
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

/// Mount the volume that contains `file`
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
