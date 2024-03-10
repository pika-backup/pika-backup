use crate::ui::prelude::*;

use super::display;
use crate::borg;
use crate::config;
use crate::schedule;
use crate::ui;
use crate::ui::utils::repo_cache::RepoCache;

pub async fn refresh_archives(
    config: config::Backup,
    from_schedule: Option<schedule::DueCause>,
) -> Result<()> {
    info!("Refreshing archives cache");
    let guard = QuitGuard::default();

    if Some(true) == REPO_CACHE.load().get(&config.repo_id).map(|x| x.reloading) {
        info!("Aborting archives cache reload because already in progress");
        return Ok(());
    } else {
        REPO_CACHE.update(|repos| {
            repos
                .entry(config.repo_id.clone())
                .or_insert_with_key(RepoCache::new)
                .reloading = true;
        });
    }
    display::ui_update_archives_spinner();

    let mut command =
        borg::Command::<borg::task::List>::new(config.clone()).set_from_schedule(from_schedule);
    command.task.set_limit_first(100);
    let result = ui::utils::borg::exec(command, &guard)
        .await
        .into_message(gettext("Failed to refresh archives cache."));

    REPO_CACHE.update(|repos| {
        repos
            .entry(config.repo_id.clone())
            .or_insert_with_key(RepoCache::new)
            .reloading = false;
    });

    display::ui_update_archives_spinner();

    let archives = result?;

    REPO_CACHE.update(enclose!((config) move |repos| {
        let repo_archives = repos
            .entry(config.repo_id.clone())
            .or_insert_with_key(RepoCache::new);

        repo_archives.archives = Some(
            archives
                .iter()
                .map(|x| (x.name.clone(), x.clone()))
                .collect(),
        );

    }));
    info!("Archives cache refreshed");

    RepoCache::write(&config.repo_id)?;

    display::ui_display_archives(&config.repo_id);

    Ok(())
}
