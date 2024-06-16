use crate::ui::prelude::*;
use crate::ui::widget::ArchivePrefixDialog;
use crate::ui::widget::CheckDialog;
use adw::prelude::*;
use adw::subclass::prelude::*;

use super::imp;
use crate::borg;
use crate::ui;

impl imp::ArchivesPage {
    pub async fn check(&self) -> Result<()> {
        let configs = BACKUP_CONFIG.load();
        let config = configs.active()?;

        let dialog = CheckDialog::new(config.id.clone());
        dialog.set_visible(true);

        Ok(())
    }

    pub async fn cleanup(&self) -> Result<()> {
        let configs = BACKUP_CONFIG.load();
        let config = configs.active()?;

        ui::widget::dialog::PruneDialog::ask_prune(&self.obj().app_window(), config).await
    }

    pub async fn edit_prefix(&self) -> Result<()> {
        let configs = BACKUP_CONFIG.load();
        let config = configs.active()?.id.clone();

        let dialog = ArchivePrefixDialog::new(config);
        dialog.present(&*self.obj());

        Ok(())
    }

    pub async fn eject(&self) -> Result<()> {
        let repo_id = BACKUP_CONFIG.load().active()?.repo_id.clone();
        ui::utils::borg::unmount(&repo_id).await?;

        Ok(())
    }

    pub async fn eject_button_clicked(&self) -> Result<()> {
        self.eject().await?;
        self.update_eject_button().await
    }

    pub async fn browse_archive(&self, archive_name: borg::ArchiveName) -> Result<()> {
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

            main_ui().page_detail().show_pending_menu(true);

            let mount = ui::utils::borg::exec(
                borg::Command::<borg::task::Mount>::new(config.clone()),
                &guard,
            )
            .await;

            if mount.is_err() {
                ACTIVE_MOUNTS.update(|mounts| {
                    mounts.remove(repo_id);
                });
                main_ui().page_detail().show_pending_menu(false);
            }

            mount.into_message(gettext("Failed to make archives available for browsing."))?;
        }

        self.update_eject_button().await?;

        let first_populated_dir = ui::utils::spawn_thread("open_archive", move || {
            super::find_first_populated_dir(&path)
        })
        .await?;

        self.show_dir(&first_populated_dir).await
    }

    pub fn delete_archive(
        &self,
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
        let window = self.obj().app_window();

        ui::widget::dialog::DeleteArchiveDialog::new(config).present_with_archive(
            &window,
            archive_name,
            archive_date,
        );

        Ok(())
    }
}
