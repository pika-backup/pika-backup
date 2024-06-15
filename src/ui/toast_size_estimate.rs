use crate::borg;
use crate::config;
use crate::ui;
use crate::ui::prelude::*;

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

        let history_save_result = BACKUP_HISTORY.try_update(clone!(@strong config.id as config_id, @strong estimate.unreadable_paths as paths => move |history| {
            if let Ok(history) = history.try_get_mut(&config_id) {
                history.set_suggested_excludes_from_absolute(config::history::SuggestedExcludeReason::PermissionDenied, paths.clone());
            }

            Ok(())
        })).await;

        if let Err(err) = history_save_result {
            err.show().await;
        }

        let space_avail = ui::utils::df::cached_or_lookup(config)
            .await
            .map(|x| x.avail);

        if let Some(space_avail) = space_avail {
            if estimate.changed > space_avail {
                let message = gettextf(
                    "Backup location “{}” might be filling up. Estimated space missing to store all data: {}.",
                    &[
                        &config.repo.location(),
                        &glib::format_size(estimate.changed - space_avail),
                    ],
                );

                ui::utils::show_notice(message);
            }
        }
    }
}
