use crate::borg;
use crate::config;
use crate::config::history;
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

    if let Some(pre_backup_command) = config.repo.settings().and_then(|s| s.pre_backup_command) {
        let pre_env = crate::ui::utils::scripts::script_env_pre(&config, from_schedule.is_some());
        crate::ui::utils::scripts::run_script(&pre_backup_command, pre_env).await?;
    }

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

    if let Some(post_backup_command) = config.repo.settings().and_then(|s| s.post_backup_command) {
        let post_env =
            crate::ui::utils::scripts::script_env_post(&config, from_schedule.is_some(), &run_info);
        crate::ui::utils::scripts::run_script(&post_backup_command, post_env).await?;
    }

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
