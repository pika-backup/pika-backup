use crate::ui::prelude::*;

use crate::borg;
use crate::config;
use crate::ui;

/// checks if there is any running backup
pub fn is_backup_running() -> bool {
    !BACKUP_COMMUNICATION.load().is_empty()
}

pub async fn exec<P: core::fmt::Display, F, V>(
    purpose: P,
    config: config::Backup,
    task: F,
) -> CombinedResult<V>
where
    F: FnOnce(borg::Borg) -> borg::Result<V> + Send + Clone + 'static + Sync,
    V: Send + 'static,
{
    let config =
        crate::ui::dialog_device_missing::updated_config(config, &purpose.to_string()).await?;
    let borg = borg::Borg::new(config);
    spawn(purpose, borg, task).await.map_err(Into::into)
}

async fn spawn<P: core::fmt::Display, F, V>(name: P, borg: borg::Borg, task: F) -> CombinedResult<V>
where
    F: FnOnce(borg::Borg) -> borg::Result<V> + Send + Clone + 'static + Sync,
    V: Send + 'static,
{
    let config = borg.get_config();

    let result = spawn_borg_thread(name, borg, task, false).await;

    if let Ok((_, ref x)) = result {
        if let Err(Error::Message(err)) =
            crate::ui::utils::secret_service::store_password(&config, x)
        {
            err.show();
        }
    }
    result.map(|(x, _)| x)
}

pub async fn only_repo_suggest_store<P: core::fmt::Display, F, V, B>(
    name: P,
    borg: B,
    task: F,
) -> CombinedResult<(V, Option<(config::Password, bool)>)>
where
    F: FnOnce(B) -> borg::Result<V> + Send + Clone + 'static + Sync,
    V: Send + 'static,
    B: borg::BorgBasics + 'static,
{
    spawn_borg_thread(name, borg, task, true).await
}

fn set_scheduler_priority(priority: i32) {
    debug!("Setting scheduler priority to {}", priority);
    let result = unsafe { nix::libc::setpriority(nix::libc::PRIO_PROCESS, 0, priority) };
    if result != 0 {
        warn!("Failed to set scheduler priority: {}", result);
    }
}

#[allow(clippy::type_complexity)]
async fn spawn_borg_thread<P: core::fmt::Display, F, V, B>(
    name: P,
    mut borg: B,
    task: F,
    mut pre_select_store: bool,
) -> CombinedResult<(V, Option<(config::Password, bool)>)>
where
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
                Err(e)
                    if matches!(e, borg::Error::PasswordMissing)
                        || e.has_borg_msgid(&borg::msg::MsgId::PassphraseWrong) =>
                {
                    if let Some((password, store)) =
                        crate::ui::utils::secret_service::get_password(pre_select_store).await
                    {
                        pre_select_store = store;
                        borg.set_password(password);

                        continue;
                    } else {
                        Err(Error::UserCanceled.into())
                    }
                }
                Err(e) if e.has_borg_msgid(&borg::msg::MsgId::LockTimeout) => {
                    handle_lock(borg.clone()).await?;
                    continue;
                }
                Err(e) => Err(e.into()),
                Ok(result) => Ok((result, borg.get_password().map(|p| (p, pre_select_store)))),
            },
        };
    }
}

async fn handle_lock<B: borg::BorgBasics + 'static>(borg: B) -> CombinedResult<()> {
    ui::utils::ConfirmationDialog::new(
        &gettext("Repository already in use."),
        &gettext("The backup repository is marked as already in use. This information can be outdated if, for example, your computer lost power while using the respository.\n\nOnly continue if you know that the respository is not used by any program! Continuing while an other program uses the repository might corrupt backup data!"),
        &gettext("Cancel"),
        &gettext("Continue Anyway"),
    )
    .set_destructive(true)
    .ask()
    .await?;
    super::spawn_thread("xxxxx", move || borg.break_lock())
        .await
        .map_err(|_| borg::Error::ThreadPanicked)?
        .map_err(Into::into)
}
