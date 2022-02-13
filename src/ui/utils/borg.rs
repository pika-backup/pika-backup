use crate::ui::prelude::*;

use crate::borg;
use crate::config;
use crate::ui;
use ui::error::Combined;

/// checks if there is any running backup
pub fn is_backup_running() -> bool {
    !BACKUP_COMMUNICATION.load().is_empty()
}

pub async fn exec<P: core::fmt::Display + Clone, F, V>(
    purpose: P,
    config: config::Backup,
    task: F,
) -> CombinedResult<V>
where
    F: FnOnce(borg::Borg) -> borg::Result<V> + Send + Clone + 'static + Sync,
    V: Send + 'static,
{
    let repo_id = config.repo_id.clone();
    if let Some(current_purpose) = BACKUP_LOCK.load().get(&repo_id) {
        return Err(Combined::Ui(
            Message::new(gettext("Repository already in use"), current_purpose).into(),
        ));
    }
    let config =
        crate::ui::dialog_device_missing::updated_config(config, &purpose.to_string()).await?;
    let borg = borg::Borg::new(config);

    BACKUP_LOCK.update(enclose!((repo_id, purpose) move |x| {
        x.insert(repo_id.clone(), purpose.to_string());
    }));

    let result = spawn_borg_thread_ask_password(purpose, borg, task).await;

    BACKUP_LOCK.update(move |x| {
        x.remove(&repo_id);
    });

    result
}

pub async fn only_repo_suggest_store<P: core::fmt::Display, F, V, B>(
    name: P,
    borg: B,
    task: F,
) -> CombinedResult<V>
where
    F: FnOnce(B) -> borg::Result<V> + Send + Clone + 'static + Sync,
    V: Send + 'static,
    B: borg::BorgBasics + 'static,
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

async fn spawn_borg_thread_ask_password<P, F, V, B>(
    name: P,
    mut borg: B,
    task: F,
) -> CombinedResult<V>
where
    P: core::fmt::Display,
    F: FnOnce(B) -> borg::Result<V> + Send + Clone + 'static + Sync,
    V: Send + 'static,
    B: borg::BorgBasics + 'static,
{
    let mut password_changed = false;

    loop {
        let result = spawn_borg_thread(name.to_string(), borg.clone(), task.clone()).await;

        return match result {
            Err(Combined::Borg(borg::Error::PasswordMissing))
            | Err(Combined::Borg(borg::Error::Failed(borg::Failure::PassphraseWrong))) => {
                if let Some(password) =
                    crate::ui::utils::secret_service::password(name.to_string()).await
                {
                    borg.set_password(password);
                    password_changed = true;

                    continue;
                } else {
                    Err(Error::UserCanceled.into())
                }
            }
            _ => {
                if password_changed {
                    if let (Some(password), Some(config)) = (&borg.password(), &borg.try_config()) {
                        if let Err(Error::Message(err)) =
                            crate::ui::utils::secret_service::store_password(config, password)
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

async fn spawn_borg_thread<P, F, V, B>(name: P, borg: B, task: F) -> CombinedResult<V>
where
    P: core::fmt::Display,
    F: FnOnce(B) -> borg::Result<V> + Send + Clone + 'static + Sync,
    V: Send + 'static,
    B: borg::BorgBasics + 'static,
{
    loop {
        let result = super::spawn_thread(
            name.to_string(),
            enclose!((borg, task) move || {
                set_scheduler_priority(10);
                task(borg)
            }),
        )
        .await;

        return match result {
            Err(futures::channel::oneshot::Canceled) => Err(borg::Error::ThreadPanicked.into()),
            Ok(result) => match result {
                Err(borg::Error::Failed(borg::Failure::LockTimeout)) => {
                    handle_lock(borg.clone()).await?;
                    continue;
                }
                Err(e) => Err(e.into()),
                Ok(result) => Ok(result),
            },
        };
    }
}

async fn handle_lock<B: borg::BorgBasics + 'static>(borg: B) -> CombinedResult<()> {
    ui::utils::ConfirmationDialog::new(
        &gettext("Repository already in use."),
        &gettext("The backup repository is marked as already in use. This information can be outdated if, for example, the computer lost power while using the repository.\n\nOnly continue if it is certain that the repository is not used by any program! Continuing while another program uses the repository might corrupt backup data!"),
        &gettext("Cancel"),
        &gettext("Continue Anyway"),
    )
    .set_destructive(true)
    .ask()
    .await?;
    super::spawn_thread("borg_break_lock", move || borg.break_lock())
        .await
        .map_err(|_| borg::Error::ThreadPanicked)?
        .map_err(Into::into)
}
