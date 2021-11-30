use adw::prelude::*;

use crate::borg;
use crate::config;
use crate::config::history;
use crate::ui;

use crate::ui::prelude::*;

use super::display;

pub async fn start_backup(config: config::Backup) -> Result<()> {
    gtk_app().hold();
    let result = startup_backup(config).await;
    gtk_app().release();
    result
}

async fn startup_backup(config: config::Backup) -> Result<()> {
    let is_running_on_repo = BACKUP_COMMUNICATION.get().keys().any(|id| {
        BACKUP_CONFIG
            .load()
            .get_result(id)
            .map(|config| &config.repo_id)
            == Ok(&config.repo_id)
    });
    if is_running_on_repo {
        return Err(Message::short(gettext("Backup on repository already running.")).into());
    }

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
        borg::Borg::umount(&config.repo_id).err_to_msg(gettext("Failed to unmount repository."))?;

        ACTIVE_MOUNTS.update(|mounts| {
            mounts.remove(&config.repo_id);
        });
    }

    let config_id = config.id.clone();
    let result = run_backup(config).await;

    BACKUP_COMMUNICATION.update(|c| {
        c.remove(&config_id);
    });

    // Direct visual feedback
    display::refresh_status();

    result
}

async fn run_backup(config: config::Backup) -> Result<()> {
    let communication: borg::Communication = Default::default();

    BACKUP_COMMUNICATION.update(|x| {
        x.insert(config.id.clone(), communication.clone());
    });
    display::refresh_status();

    BACKUP_HISTORY.update(|history| {
        history.set_running(config.id.clone());
    });
    ui::write_config()?;

    // estimate backup size if not running in background
    if crate::ui::app_window::is_displayed() {
        communication
            .status
            .update(|status| status.run = borg::Run::SizeEstimation);

        let config = crate::ui::dialog_device_missing::updated_config(
            config.clone(),
            &gettext("Creating new backup."),
        )
        .await?;
        let estimated_size = ui::utils::spawn_thread(
            "estimate_backup_size",
            enclose!((config, communication) move ||
                borg::size_estimate::calculate(&config, &communication)
            ),
        )
        .await
        .ok()
        .flatten();

        if estimated_size.is_some() {
            communication.status.update(move |status| {
                status.estimated_size = estimated_size.clone();
            });
        }
    }

    let estimated_changed = communication
        .status
        .load()
        .estimated_size
        .as_ref()
        .map(|x| x.changed);
    let space_avail = ui::utils::df::cached_or_lookup(&config)
        .await
        .map(|x| x.avail);

    if let (Some(estimated_changed), Some(space_avail)) = (estimated_changed, space_avail) {
        if estimated_changed > space_avail {
            ui::utils::show_notice(gettextf(
                "Backup location “{}” might be filling up. Estimated space missing to store all data: {}.",
                &[
                    &config.repo.location(),
                    &glib::format_size(estimated_changed - space_avail),
                ],
            ));
        }
    }

    // execute backup
    let result = ui::utils::borg::exec(
        gettext("Creating new backup."),
        config.clone(),
        enclose!((communication) move |borg| borg.create(communication)),
    )
    .await;

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

    let message_history = communication.status.load().all_combined_message_history();

    let run_info = history::RunInfo::new(&config, outcome, message_history);

    BACKUP_HISTORY.update(|history| {
        history.insert(config.id.clone(), run_info.clone());
        history.remove_running(config.id.clone());
    });
    display::refresh_status();

    ui::write_config()?;

    match result {
        Err(borg::Error::Aborted(_)) => Ok(()),
        Err(err) => Err(Message::new(gettext("Creating a backup failed."), err).into()),
        Ok(_)
            if run_info.messages.clone().filter_handled().max_log_level()
                >= Some(borg::msg::LogLevel::Warning) =>
        {
            Err(Message::new(
                gettext("Backup completed with warnings."),
                run_info.messages.filter_hidden().to_string(),
            )
            .into())
        }
        Ok(_) => {
            // TODO: Should happen with warnings as well
            let _ignore = ui::page_archives::refresh_archives_cache(config.clone()).await;
            let _ignore = ui::utils::df::lookup_and_cache(&config).await;
            Ok(())
        }
    }
}
