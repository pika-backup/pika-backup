use crate::ui::prelude::*;
use crate::{borg, config, ui};

pub async fn check(
    config: &config::Backup,
    communication: borg::Communication<borg::task::Create>,
) {
    let estimated_size = ui::utils::spawn_thread(
        "estimate_backup_size",
        enclose!((config, communication) move ||
            borg::size_estimate::calculate(&config, &BACKUP_HISTORY.load(), &communication)
        ),
    )
    .await
    .ok()
    .flatten();

    if let Some(estimate) = &estimated_size {
        communication
            .specific_info
            .update(enclose!((estimated_size) move |status| {
                status.estimated_size.clone_from(&estimated_size);
            }));

        let history_save_result = BACKUP_HISTORY
            .try_update(clone!(
                #[strong(rename_to = config_id)]
                config.id,
                #[strong(rename_to = paths)]
                estimate.unreadable_paths,
                move |history| {
                    if let Ok(history) = history.try_get_mut(&config_id) {
                        history.set_suggested_excludes_from_absolute(
                            config::history::SuggestedExcludeReason::PermissionDenied,
                            paths.clone(),
                        );
                    }

                    Ok(())
                }
            ))
            .await;

        if let Err(err) = history_save_result {
            err.show().await;
        }

        let space_avail = ui::utils::df::cached_or_lookup(config)
            .await
            .map(|x| x.avail);

        if let Some(space_avail) = space_avail
            && estimate.changed > space_avail
        {
            let message = gettextf(
                "Backup location “{}” might be filling up. Estimated space missing to store all data: {}.",
                [
                    config.repo.location().as_str(),
                    &glib::format_size(estimate.changed - space_avail),
                ],
            );

            ui::utils::show_notice(message);
        }
    }
}
