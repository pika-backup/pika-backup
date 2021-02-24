use crate::ui::prelude::*;

use crate::borg;
use crate::config::Password;

/// checks if there is any running backup
pub fn is_backup_running() -> bool {
    !BACKUP_COMMUNICATION.load().is_empty()
}

pub async fn spawn<F, V>(name: &'static str, borg: borg::Borg, task: F) -> borg::Result<V>
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

pub async fn only_repo_suggest_store<F, V, B>(
    name: &'static str,
    borg: B,
    task: F,
) -> borg::Result<(V, Option<(Password, bool)>)>
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
async fn spawn_borg_thread<F, V, B>(
    name: &'static str,
    mut borg: B,
    task: F,
    mut pre_select_store: bool,
) -> borg::Result<(V, Option<(Password, bool)>)>
where
    F: FnOnce(B) -> borg::Result<V> + Send + Clone + 'static + Sync,
    V: Send + 'static,
    B: borg::BorgBasics + 'static,
{
    loop {
        let result = super::spawn_thread(
            name,
            enclose!((borg, task) move || {
                set_scheduler_priority(10);
                task(borg)
            }),
        )
        .await;

        return match result {
            Err(futures::channel::oneshot::Canceled) => Err(borg::Error::ThreadPanicked),
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
                        Err(borg::Error::UserAborted)
                    }
                }
                Err(e) => Err(e),
                Ok(result) => Ok((result, borg.get_password().map(|p| (p, pre_select_store)))),
            },
        };
    }
}
