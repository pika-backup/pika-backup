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
                status.estimated_size = estimated_size.clone();
            }));

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
