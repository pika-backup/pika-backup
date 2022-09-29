use crate::ui::prelude::*;

use crate::borg;
use crate::ui;
use borg::task::Task;
use std::future::Future;
use ui::error::Combined;

// TODO: this does no really check for backups
/// checks if there is any running backup
pub fn is_backup_running() -> bool {
    !BORG_OPERATION.with(|op| op.load().is_empty())
}

pub async fn exec<T: Task>(mut command: borg::Command<T>) -> CombinedResult<T::Return>
where
    borg::Command<T>: borg::CommandRun<T>,
{
    let config_id = command.config.id.clone();

    command.config =
        crate::ui::dialog_device_missing::updated_config(&command.config, &T::name()).await?;

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

    let result = spawn_borg_thread_ask_password(command).await;

    BORG_OPERATION.with(move |operations| {
        operations.update(|op| {
            op.remove(&config_id);
        });
    });

    result
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
            Err(Combined::Borg(borg::Error::PasswordMissing))
            | Err(Combined::Borg(borg::Error::Failed(borg::Failure::PassphraseWrong))) => {
                if let Some(password) =
                    crate::ui::utils::password_storage::password(command.repo(), T::name()).await
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
                            err.show().await;
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
    .map_err(Into::into)
}

#[async_std::test]
async fn test_exec_operation_register() {
    gtk::init().unwrap();

    let mut config = crate::config::Backup::test_new_mock();
    config.schedule.frequency = crate::config::Frequency::Hourly;

    let command = borg::Command::<borg::task::List>::new(config)
        .set_from_schedule(Some(crate::schedule::DueCause::Regular));

    assert!(!is_backup_running());
    assert!(exec(command.clone()).await.is_err());
    assert!(!is_backup_running());
}
