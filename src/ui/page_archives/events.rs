use crate::ui::prelude::*;
use adw::prelude::*;

use super::display;
use crate::borg;
use crate::ui;

pub async fn cleanup() -> Result<()> {
    let configs = BACKUP_CONFIG.load();
    let config = configs.active()?;

    ui::dialog_prune::run(config).await
}

pub async fn eject_button_clicked() -> Result<()> {
    let repo_id = BACKUP_CONFIG.load().active()?.repo_id.clone();

    borg::Borg::umount(&repo_id).err_to_msg(gettext("Failed to unmount repository."))?;
    ACTIVE_MOUNTS.update(|mounts| {
        mounts.remove(&repo_id);
    });
    display::update_eject_button()
}

pub async fn browse_archive(archive_name: borg::ArchiveName) -> Result<()> {
    let configs = BACKUP_CONFIG.load();
    let config = configs.active()?;
    let repo_id = &config.repo_id;

    debug!("Trying to browse an archive");

    let backup_mounted = ACTIVE_MOUNTS.load().contains(repo_id);

    let mut path = borg::Borg::mount_point(repo_id);
    path.push(archive_name.as_str());

    if !backup_mounted {
        ACTIVE_MOUNTS.update(|mounts| {
            mounts.insert(repo_id.clone());
        });

        main_ui().pending_menu().show();
        let mount = ui::utils::borg::exec(gettext("Browse Archive"), config.clone(), move |borg| {
            borg.mount()
        })
        .await;

        if mount.is_err() {
            ACTIVE_MOUNTS.update(|mounts| {
                mounts.remove(repo_id);
            });
            main_ui().pending_menu().hide();
        }

        mount.into_message(gettext("Failed to make archives available for browsing."))?;
    }

    display::update_eject_button()?;

    let first_populated_dir = ui::utils::spawn_thread("open_archive", move || {
        super::find_first_populated_dir(&path)
    })
    .await?;

    display::show_dir(&first_populated_dir)
}
