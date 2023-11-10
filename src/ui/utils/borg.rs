use crate::borg::task;
use crate::borg::RepoId;
use crate::ui::prelude::*;

use crate::borg;
use crate::ui;
use borg::task::Task;
use gio::traits::DriveExt;
use gio::traits::VolumeExt;
use std::future::Future;
use ui::error::Combined;

/// Is a borg operation registered with a [QuitGuard]]?
pub fn is_borg_operation_running() -> bool {
    STATUS_TRACKING.with(|status| status.quit_inhibit_count() > 0)
}

/// Checks whether borg command line arguments are valid
pub fn parse_borg_command_line_args(text: &str) -> Result<Vec<String>> {
    if let Ok(args) = shell_words::split(text) {
        Ok(args)
    } else {
        Err(Message::new(
            gettext("Additional command line arguments invalid"),
            gettext("Please check for missing closing quotes."),
        )
        .into())
    }
}

/// Executes a borg command
///
/// This takes a [QuitGuard] to prove that one has been set up and is currently active.
pub async fn exec<T: Task>(
    mut command: borg::Command<T>,
    _guard: &QuitGuard,
) -> CombinedResult<T::Return>
where
    borg::Command<T>: borg::CommandRun<T>,
{
    let config_id = command.config.id.clone();

    if T::KIND != borg::task::Kind::Mount {
        // If a repository is mounted we ask to unmount it before we continue
        ask_unmount(T::KIND, &command.config.repo_id).await?;
    }

    BORG_OPERATION.with(enclose!((command) move |operations| {
        if let Some(operation) = operations
            .load()
            .values()
            .find(|x| x.repo_id() == &command.config.repo_id)
        {
            return Err(Combined::Ui(
                Message::new(gettext("Repository already in use"), operation.name()).into(),
            ));
        }

        ui::operation::Operation::register(command);

        Ok(())
    }))?;

    BACKUP_HISTORY.update(enclose!((config_id) move |history| {
        history.set_running(config_id.clone());
    }));

    Handler::handle(ui::write_config());

    scopeguard::defer_on_success! {
        BORG_OPERATION.with(enclose!((config_id) move |operations| {
            operations.update(|op| {
                op.remove(&config_id);
            });
        }));

        BACKUP_HISTORY.update(move |history| {
            history.remove_running(config_id.clone());
        });

        Handler::handle(ui::write_config());
    };

    let mounted_result =
        crate::ui::dialog_device_missing::ensure_repo_available(&command.config, &T::name()).await;

    match mounted_result {
        Ok(config) => command.config = config,
        Err(err) => {
            // The repository is not available after trying to mount it
            return Err(
                borg::Error::Aborted(borg::Abort::RepositoryNotAvailable(err.to_string())).into(),
            );
        }
    }

    spawn_borg_thread_ask_password(command).await
}

pub async fn exec_repo_only<P: core::fmt::Display, F, R, V>(
    name: P,
    borg: borg::CommandOnlyRepo,
    task: F,
) -> CombinedResult<V>
where
    F: FnOnce(borg::CommandOnlyRepo) -> R + Send + Clone + 'static + Sync,
    R: Future<Output = borg::Result<V>>,
    V: Send + 'static,
    //B: borg::BorgBasics + 'static,
{
    spawn_borg_thread(name, borg, task).await
}

async fn ask_unmount(kind: task::Kind, repo_id: &RepoId) -> Result<()> {
    crate::ui::utils::borg::cleanup_mounts().await?;

    if ACTIVE_MOUNTS.load().contains(repo_id) {
        debug!("Trying to run a {kind:?} on a backup that is currently mounted.");

        match kind {
            task::Kind::Create => {
                ui::utils::confirmation_dialog(
                    &gettext("Stop browsing files and start backup?"),
                    &gettext(
                        "Browsing through archived files is not possible while running a backup.",
                    ),
                    &gettext("Keep Browsing"),
                    &gettext("Start Backup"),
                )
                .await?;
            }
            _ => {
                ui::utils::confirmation_dialog(
                    &gettext("Stop browsing files and start operation?"),
                    &gettext(
                        "Browsing through archived files is not possible while running an operation on the repository.",
                    ),
                    &gettext("Keep Browsing"),
                    &gettext("Start Operation"),
                )
                .await?;
            }
        }

        trace!("User decided to unmount repo.");
        unmount(repo_id)
            .await
            .err_to_msg(gettext("Failed to unmount repository."))?;
    }

    Ok(())
}

fn set_scheduler_priority(priority: i32) {
    debug!("Setting scheduler priority to {}", priority);
    let result = unsafe { nix::libc::setpriority(nix::libc::PRIO_PROCESS, 0, priority) };
    if result != 0 {
        warn!("Failed to set scheduler priority: {}", result);
    }
}

async fn spawn_borg_thread_ask_password<C: 'static + borg::CommandRun<T>, T: Task>(
    mut command: C,
) -> CombinedResult<T::Return> {
    let mut password_changed = false;

    loop {
        let result = spawn_borg_thread(T::name(), command.clone(), |x| x.run()).await;

        return match result {
            Err(Combined::Borg(borg::Error::PasswordMissing { .. }))
            | Err(Combined::Borg(borg::Error::Failed(borg::Failure::PassphraseWrong))) => {
                let keyring_error =
                    if let Err(Combined::Borg(borg::Error::PasswordMissing { keyring_error })) =
                        result
                    {
                        keyring_error
                    } else {
                        None
                    };

                if let Some(password) = crate::ui::utils::password_storage::password_dialog(
                    command.repo(),
                    T::name(),
                    keyring_error,
                )
                .await
                {
                    command.set_password(password);
                    password_changed = true;

                    continue;
                } else {
                    Err(Error::UserCanceled.into())
                }
            }
            _ => {
                if password_changed {
                    if let (Some(password), Some(config)) =
                        (&command.password(), &command.try_config())
                    {
                        if let Err(Error::Message(err)) =
                            crate::ui::utils::password_storage::store_password(config, password)
                                .await
                        {
                            warn!("Error using keyring, using in-memory password store. Keyring error: '{err:?}'");

                            // Use the in-memory password store instead
                            crate::globals::MEMORY_PASSWORD_STORE
                                .set_password(config, password.clone());
                        }

                        if !config.encrypted {
                            // We assumed that the repo doesn't have encryption, but this assumption was outdated.
                            // Set the encrypted flag in the config
                            if let Some(id) = command.config_id() {
                                BACKUP_CONFIG.try_update(|config| {
                                    let cfg = config.try_get_mut(&id)?;
                                    cfg.encrypted = true;

                                    Ok(())
                                })?;

                                ui::write_config()?;
                            }
                        }
                    }
                }
                result
            }
        };
    }
}

async fn spawn_borg_thread<P, F, R, V, B>(name: P, borg: B, task: F) -> CombinedResult<V>
where
    P: core::fmt::Display,
    F: FnOnce(B) -> R + Send + Clone + 'static + Sync,
    R: Future<Output = borg::Result<V>>,
    V: Send + 'static,
    B: borg::BorgRunConfig,
{
    loop {
        let result = super::spawn_thread(
            name.to_string(),
            enclose!((borg, task) move || {
                set_scheduler_priority(10);
                async_std::task::block_on(task(borg))
            }),
        )
        .await;

        return match result? {
            Err(borg::Error::Failed(borg::Failure::LockTimeout)) => {
                handle_lock(borg.clone()).await?;
                continue;
            }
            Err(e) => Err(e.into()),
            Ok(result) => Ok(result),
        };
    }
}

async fn handle_lock<B: borg::BorgRunConfig>(borg: B) -> CombinedResult<()> {
    ui::utils::ConfirmationDialog::new(
        &gettext("Repository already in use."),
        &(gettext("The backup repository is marked as already in use. This information can be outdated if, for example, the computer lost power while using the repository.")
        + "\n\n"
        + &gettext("Only continue if it is certain that the repository is not used by any program! Continuing while another program uses the repository might corrupt backup data!")),
        &gettext("Cancel"),
        &gettext("Continue Anyway"),
    )
    .set_destructive(true)
    .ask()
    .await?;

    super::spawn_thread("borg_break_lock", move || {
        borg::CommandOnlyRepo::new(borg.repo()).break_lock()
    })
    .await
    .map_err(|_| borg::Error::ThreadPanicked)?
    .await
    .map_err(Into::into)
}

pub async fn unmount(repo_id: &RepoId) -> Result<()> {
    borg::functions::umount(repo_id)
        .await
        .err_to_msg(gettext("Failed to unmount repository."))?;
    ACTIVE_MOUNTS.update(|mounts| {
        mounts.remove(repo_id);
    });

    crate::ui::page_archives::refresh_status();

    Ok(())
}

pub async fn cleanup_mounts() -> Result<()> {
    let mounts = ACTIVE_MOUNTS.load();

    // Find mounts that were already unmounted outside of Pika
    for repo_id in mounts.iter() {
        if !borg::is_mounted(repo_id).await {
            // The repository was unmounted somewhere else
            // Call unmount to fix the state
            warn!("Marking repo {repo_id:?} as unmounted, as the mountpoint doesn't exist anymore");
            unmount(repo_id).await?;
        }
    }

    // Find mounts that should belong to Pika but aren't registered.
    // This would be leftover mounts from a previous run of Pika when it wasn't quit properly.
    for config in BACKUP_CONFIG.load().iter() {
        let repo_id = &config.repo_id;
        if !mounts.contains(repo_id) && borg::is_mounted(repo_id).await {
            warn!(
                "Marking repo {repo_id:?} as mounted, was probably mounted from a force-quit app"
            );
            ACTIVE_MOUNTS.update(|mounts| {
                mounts.insert(repo_id.clone());
            });
        }
    }

    Ok(())
}

pub async fn unmount_backup_disk(backup: crate::config::Backup) -> Result<()> {
    if let Some(volume) = backup.repo.removable_drive_volume() {
        // We have a removable drive and found a volume
        let mount_operation = gtk::MountOperation::new(Some(&main_ui().window()));

        if let Some(drive) =
            volume
                .drive()
                .and_then(|drive| if drive.can_eject() { Some(drive) } else { None })
        {
            // We don't need to stop the drive, it will only spin down the hard disk. The drive is safe to remove in any case
            let res = if drive.can_stop() {
                debug!("Stopping drive {}", drive.name());
                drive
                    .stop_future(gio::MountUnmountFlags::empty(), Some(&mount_operation))
                    .await
            } else {
                debug!("Ejecting drive {}", drive.name());
                drive
                    .eject_with_operation_future(
                        gio::MountUnmountFlags::empty(),
                        Some(&mount_operation),
                    )
                    .await
            };

            if let Err(err) = res {
                if let Some(gio::IOErrorEnum::FailedHandled) = err.kind() {
                    debug!("Unmount aborted by user: {}", err);
                    return Ok(());
                } else {
                    debug!("Error ejecting disk: {}", err);
                    return Err(Message::new(
                        gettext("Unable to Eject Backup Disk"),
                        err.to_string(),
                    )
                    .into());
                }
            }

            // When the drive was ejected we can show a toast
            let toast = adw::Toast::builder()
                .title(gettextf("{} can be safely unplugged.", &[&drive.name()]))
                .timeout(5)
                .build();

            main_ui().toast().add_toast(toast);
        } else {
            debug!(
                "Unmount disk: Backup disk {} can't be ejected",
                volume.name()
            );
            Err(Message::new(
                gettext("Unable to Eject Backup Disk"),
                gettextf("{} can't be ejected.", &[&volume.name()]),
            ))?;
        }
    } else {
        debug!("Unmount disk: Backup disk not found");
    }

    Ok(())
}

#[async_std::test]
async fn test_exec_operation_register() {
    gtk::init().unwrap();

    let mut config = crate::config::Backup::test_new_mock();
    config.schedule.frequency = crate::config::Frequency::Hourly;

    let command = borg::Command::<borg::task::List>::new(config)
        .set_from_schedule(Some(crate::schedule::DueCause::Regular));

    assert!(!is_borg_operation_running());

    {
        let guard = QuitGuard::default();
        assert!(is_borg_operation_running());

        assert!(exec(command.clone(), &guard).await.is_err());
        assert!(is_borg_operation_running());
    }

    assert!(!is_borg_operation_running());
}
