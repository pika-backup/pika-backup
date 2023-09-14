use crate::ui::dialog_check::DialogCheck;
use crate::ui::prelude::*;
use adw::prelude::*;

use super::display;
use crate::borg;
use crate::ui;

pub async fn check() -> Result<()> {
    let configs = BACKUP_CONFIG.load();
    let config = configs.active()?;

    let dialog = DialogCheck::new(config.id.clone());
    dialog.set_visible(true);

    Ok(())
}

pub async fn cleanup() -> Result<()> {
    let configs = BACKUP_CONFIG.load();
    let config = configs.active()?;

    ui::dialog_prune::run(config).await
}

pub async fn edit_prefix() -> Result<()> {
    let configs = BACKUP_CONFIG.load();
    let config = configs.active()?;

    ui::dialog_archive_prefix::run(config);
    Ok(())
}

pub async fn eject() -> Result<()> {
    let repo_id = BACKUP_CONFIG.load().active()?.repo_id.clone();
    ui::utils::borg::unmount(&repo_id).await?;

    Ok(())
}

pub async fn eject_button_clicked() -> Result<()> {
    eject().await?;
    display::update_eject_button().await
}

pub async fn browse_archive(archive_name: borg::ArchiveName) -> Result<()> {
    let guard = QuitGuard::default();
    let configs = BACKUP_CONFIG.load();
    let config = configs.active()?;
    let repo_id = &config.repo_id;

    debug!("Trying to browse an archive");

    // Register mounts from a previous run that quit improperly
    crate::ui::utils::borg::cleanup_mounts().await?;

    let backup_mounted = ACTIVE_MOUNTS.load().contains(repo_id);

    let mut path = borg::functions::mount_point(repo_id);
    path.push(archive_name.as_str());

    if !backup_mounted {
        ACTIVE_MOUNTS.update(|mounts| {
            mounts.insert(repo_id.clone());
        });

        main_ui().pending_menu().set_visible(true);

        let mount = ui::utils::borg::exec(
            borg::Command::<borg::task::Mount>::new(config.clone()),
            &guard,
        )
        .await;

        if mount.is_err() {
            ACTIVE_MOUNTS.update(|mounts| {
                mounts.remove(repo_id);
            });
            main_ui().pending_menu().set_visible(false);
        }

        mount.into_message(gettext("Failed to make archives available for browsing."))?;
    }

    display::update_eject_button().await?;

    let first_populated_dir = ui::utils::spawn_thread("open_archive", move || {
        super::find_first_populated_dir(&path)
    })
    .await?;

    display::show_dir(&first_populated_dir).await
}

pub async fn delete_archive(
    archive_name: borg::ArchiveName,
    archive: borg::ListArchive,
) -> Result<()> {
    let configs = BACKUP_CONFIG.load();
    let config = configs.active()?;

    debug!("Trying to delete an archive");

    let archive_name = archive_name.as_str();
    let archive_date = &archive
        .start
        .to_locale()
        .unwrap_or_else(|| archive.start.to_string())
        .clone();

    ui::dialog_delete_archive::run(config, archive_name, archive_date).await
}
