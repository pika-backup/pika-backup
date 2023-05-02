use crate::borg;
use crate::config;
use crate::config::history;
use crate::config::history::RunInfo;
use crate::config::UserScriptKind;
use crate::schedule;
use crate::ui;

use crate::ui::prelude::*;

use super::display;

pub async fn start_backup(
    config: config::Backup,
    from_schedule: Option<schedule::DueCause>,
) -> Result<()> {
    let guard = QuitGuard::default();
    if ACTIVE_MOUNTS.load().contains(&config.repo_id) {
        debug!("Trying to run borg::create on a backup that is currently mounted.");

        ui::utils::confirmation_dialog(
            &gettext("Stop browsing files and start backup?"),
            &gettext("Browsing through archived files is not possible while running a backup."),
            &gettext("Keep Browsing"),
            &gettext("Start Backup"),
        )
        .await?;

        trace!("User decided to unmount repo.");
        borg::functions::umount(&config.repo_id)
            .err_to_msg(gettext("Failed to unmount repository."))?;

        ACTIVE_MOUNTS.update(|mounts| {
            mounts.remove(&config.repo_id);
        });
    }

    let result = run_backup(config, from_schedule, &guard).await;
    display::refresh_status();

    result
}

async fn run_prune(
    config: config::Backup,
    from_schedule: Option<schedule::DueCause>,
    guard: &QuitGuard,
) -> Result<bool> {
    let prune_command = borg::Command::<borg::task::Prune>::new(config.clone())
        .set_from_schedule(from_schedule.clone());
    let prune_result = ui::utils::borg::exec(prune_command, guard)
        .await
        .into_borg_error()?;

    match prune_result {
        Err(borg::Error::Aborted(_)) => return Ok(false),
        Err(err) => return Err(Message::new(gettext("Delete old Archives Failed"), err).into()),
        _ => {}
    };

    let compact_command = borg::Command::<borg::task::Compact>::new(config.clone());
    let compact_result = ui::utils::borg::exec(compact_command, guard)
        .await
        .into_borg_error()?;

    match compact_result {
        Err(borg::Error::Aborted(_)) => return Ok(false),
        Err(err) => return Err(Message::new(gettext("Reclaiming Free Space Failed"), err).into()),
        _ => {}
    };

    Ok(true)
}

async fn run_backup(
    config: config::Backup,
    from_schedule: Option<schedule::DueCause>,
    guard: &QuitGuard,
) -> Result<()> {
    scopeguard::defer_on_success! {
        BACKUP_HISTORY.update(|history| {
            history.remove_running(config.id.clone());
        });
        Handler::handle(ui::write_config());
    }

    BACKUP_HISTORY.update(|history| {
        history.set_running(config.id.clone());
    });
    ui::write_config()?;

    run_script(UserScriptKind::PreBackup, config.clone(), None, guard).await?;

    let command = borg::Command::<borg::task::Create>::new(config.clone())
        .set_from_schedule(from_schedule.clone());
    let communication = command.communication.clone();

    // estimate backup size if not running in background
    if crate::ui::app_window::is_displayed() {
        let config = config.clone();
        let communication = communication.clone();
        glib::MainContext::default().spawn_local(async move {
            ui::toast_size_estimate::check(&config, communication).await
        });
    }

    // execute backup
    let result = ui::utils::borg::exec(command, guard).await;

    let result = result.into_borg_error()?;

    // This is because the error cannot be cloned
    let outcome = match &result {
        Err(borg::Error::Aborted(err)) => borg::Outcome::Aborted(err.clone()),
        Err(borg::Error::Failed(err)) => borg::Outcome::Failed(err.clone()),
        Err(err) => borg::Outcome::Failed(borg::error::Failure::Other(err.to_string())),
        Ok(stats) => borg::Outcome::Completed {
            stats: stats.clone(),
        },
    };

    let message_history = communication
        .general_info
        .load()
        .all_combined_message_history();

    let run_info = history::RunInfo::new(&config, outcome, message_history);

    BACKUP_HISTORY.update(|history| {
        history.insert(config.id.clone(), run_info.clone());
        history.remove_running(config.id.clone());
    });

    ui::write_config()?;

    run_script(
        UserScriptKind::PostBackup,
        config.clone(),
        Some(run_info.clone()),
        guard,
    )
    .await?;

    match result {
        Err(borg::Error::Aborted(_)) => Ok(()),
        Err(err) => Err(Message::new(gettext("Backup Failed"), err).into()),
        Ok(_) => {
            if config.prune.enabled {
                // use current config for pruning archives
                // assuming it's closer to what users expect
                if let Ok(current_config) = BACKUP_CONFIG.load().get_result(&config.id) {
                    match run_prune(current_config.clone(), from_schedule.clone(), guard).await {
                        Ok(false) => return Ok(()),
                        Err(err) => return Err(err),
                        _ => {}
                    };
                }
            }

            let _ignore =
                ui::page_archives::cache::refresh_archives(config.clone(), from_schedule).await;
            let _ignore = ui::utils::df::lookup_and_cache(&config).await;

            if run_info.messages.clone().filter_handled().max_log_level()
                >= Some(borg::log_json::LogLevel::Warning)
            {
                Err(Message::new(
                    gettext("Backup Completed with Warnings"),
                    run_info.messages.filter_hidden().to_string(),
                )
                .into())
            } else {
                Ok(())
            }
        }
    }
}

async fn run_script(
    kind: UserScriptKind,
    config: crate::config::Backup,
    run_info: Option<crate::config::history::RunInfo>,
    guard: &QuitGuard,
) -> Result<()> {
    if config.user_scripts.get(&kind).is_none() {
        // Don't even run the task if it's not configured
        return Ok(());
    }

    let mut command = crate::borg::Command::<crate::borg::task::UserScript>::new(config.clone());
    command.task.set_kind(kind);
    command.task.set_run_info(run_info.clone());

    let result = crate::ui::utils::borg::exec(command, guard).await;
    let outcome = match &result {
        Err(crate::ui::error::Combined::Borg(borg::Error::Aborted(err))) => {
            Some(borg::Outcome::Aborted(err.clone()))
        }
        Err(crate::ui::error::Combined::Borg(borg_err)) => Some(borg::Outcome::Aborted(
            borg::Abort::UserShellCommand(borg_err.to_string()),
        )),
        _ => None,
    };

    if let Some(outcome) = outcome {
        let run_info = RunInfo::new(&config, outcome, vec![]);

        BACKUP_HISTORY.update(move |history| {
            history.insert(config.id.clone(), run_info.clone());
            history.remove_running(config.id.clone());
        });
    }

    result.into_message(gettext("Error Running Shell Command"))
}
