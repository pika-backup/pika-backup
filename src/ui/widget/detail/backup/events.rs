use std::ffi::OsStr;
use std::path::PathBuf;

use adw::prelude::*;
use adw::subclass::prelude::*;

use super::imp;
use crate::ui::prelude::*;
use crate::{borg, ui};

#[gtk::template_callbacks]
impl imp::BackupPage {
    pub async fn on_stop_backup_create(&self) -> Result<()> {
        let operation = BORG_OPERATION.with(|op| Ok::<_, Error>(op.load().active()?.clone()))?;

        // Abort immediately if only reconnecting
        if !operation.aborting() && !matches!(operation.status(), borg::Run::Reconnecting(_)) {
            match operation.task_kind() {
                borg::task::Kind::Create => {
                    ui::utils::confirmation_dialog(
                        &*self.obj(),
                        &gettext("Stop Running Backup?"),
                        &gettext("The current backup state will be saved. You can continue your backup later by starting it again."),
                        &gettext("Continue"),
                        &gettext("Stop"),
                    )
                    .await?;
                }
                borg::task::Kind::Prune | borg::task::Kind::Delete => {
                    ui::utils::confirmation_dialog(
                        &*self.obj(),
                        &gettext("Abort Delete Operation?"),
                        &gettext("Archives are currently being deleted. Free space will not be reclaimed when aborting."),
                        &gettext("Continue"),
                        &gettext("Abort"),
                    )
                    .await?;
                }
                _ => {
                    ui::utils::confirmation_dialog(
                        &*self.obj(),
                        &gettext("Abort Operation?"),
                        &gettext("An operation is currently being performed. Aborting now will cause any progress made by the operation to be lost."),
                        &gettext("Continue"),
                        &gettext("Abort"),
                    )
                    .await?;
                }
            }
        }

        operation.set_instruction(borg::Instruction::Abort(borg::Abort::User));

        Ok(())
    }

    pub async fn on_backup_run(&self, guard: &QuitGuard) -> Result<()> {
        self.backup(BACKUP_CONFIG.load().active()?.clone(), None, guard)
            .await
    }

    pub async fn on_backup_disk_eject(&self) -> Result<()> {
        // Hide the button immediately to prevent accidental multiple triggers of the
        // action It will be shown again on error
        self.backup_disk_eject_button.set_visible(false);

        let res =
            ui::utils::borg::unmount_backup_disk(BACKUP_CONFIG.load().active()?.clone()).await;
        self.refresh()?;
        res
    }

    async fn include_file_paths(&self, paths: Vec<PathBuf>) -> Result<()> {
        let paths = if *APP_IS_SANDBOXED {
            let runtime_dir = glib::user_runtime_dir();
            let mut sandbox_filtered_paths = Vec::new();
            let mut root_paths = Vec::new();

            // Scan for unavailable paths in the sandbox and redirect them if possible
            let paths = paths
                .into_iter()
                .filter(|path| {
                    // Filter all paths that are definitely unavailable and give a note about them
                    if path.starts_with(runtime_dir.join("doc/")) {
                        sandbox_filtered_paths.push(path.display().to_string());
                        false
                    } else if path.starts_with("/dev") || path == OsStr::new("/") {
                        root_paths.push(path.display().to_string());
                        false
                    } else {
                        true
                    }
                })
                .collect::<Vec<PathBuf>>();

            if !sandbox_filtered_paths.is_empty() {
                let path_list = sandbox_filtered_paths.join("\n");

                ui::utils::show_error_transient_for(
                gettext("Unable to Include Location"),
                gettextf("The following paths could not be included because they aren't reliably available in the sandbox:\n{}", &[&path_list]),
                None,
                &main_ui().window(),
            )
            .await;
            }

            if !root_paths.is_empty() {
                ui::utils::show_error_transient_for(
                gettext("Unable to Include Location"),
                gettext("Pika Backup cannot be used to backup the entire system or the “/dev” directory."),
                None,
                &main_ui().window(),
            )
            .await;
            }

            paths
        } else {
            paths
        };

        if !paths.is_empty() {
            BACKUP_CONFIG
                .try_update(|settings| {
                    for path in &paths {
                        settings
                            .active_mut()?
                            .include
                            .insert(ui::utils::rel_path(path));
                    }
                    Ok(())
                })
                .await?;

            self.refresh()?;
        }

        Ok(())
    }

    pub async fn add_include_file(&self) -> Result<()> {
        let chooser = gtk::FileDialog::builder()
            .initial_folder(&gio::File::for_path(glib::home_dir()))
            .title(gettext("Include File"))
            .accept_label(gettext("Select"))
            .modal(true)
            .build();

        let paths = ui::utils::paths_from_model(Some(
            chooser
                .open_multiple_future(Some(&main_ui().window()))
                .await
                .map_err(|err| match err.kind::<gtk::DialogError>() {
                    Some(gtk::DialogError::Cancelled | gtk::DialogError::Dismissed) => {
                        Error::UserCanceled
                    }
                    _ => Message::short(err.to_string()).into(),
                })?,
        ))?;

        self.include_file_paths(paths).await?;
        Ok(())
    }

    pub async fn add_include(&self) -> Result<()> {
        let chooser = gtk::FileDialog::builder()
            .initial_folder(&gio::File::for_path(glib::home_dir()))
            .title(gettext("Include Folder"))
            .accept_label(gettext("Select"))
            .modal(true)
            .build();

        let paths = ui::utils::paths_from_model(Some(
            chooser
                .select_multiple_folders_future(Some(&main_ui().window()))
                .await
                .map_err(|err| match err.kind::<gtk::DialogError>() {
                    Some(gtk::DialogError::Cancelled | gtk::DialogError::Dismissed) => {
                        Error::UserCanceled
                    }
                    _ => Message::short(err.to_string()).into(),
                })?,
        ))?;

        self.include_file_paths(paths).await
    }

    pub async fn add_exclude(&self) -> Result<()> {
        let config = BACKUP_CONFIG.load_full();
        let active = config.active()?;
        let window = self.obj().app_window();
        ui::widget::dialog::ExcludeDialog::new(active).present(Some(&window));

        Ok(())
    }

    pub async fn on_remove_include(&self, path: std::path::PathBuf) -> Result<()> {
        if self.confirm_remove_include(&path).await {
            BACKUP_CONFIG
                .try_update(|settings| {
                    settings.active_mut()?.include.remove(&path);
                    Ok(())
                })
                .await?;
            self.refresh()?;
        }

        Ok(())
    }

    async fn confirm_remove_include(&self, path: &std::path::Path) -> bool {
        let path_string = if path == std::path::Path::new("") {
            gettext("Home")
        } else {
            path.display().to_string()
        };

        ui::utils::confirmation_dialog(
            &*self.obj(),
            &gettextf("No longer include “{}” in backups?", &[&path_string]),
            &gettext(
                "All files contained in this folder will no longer be part of future backups.",
            ),
            &gettext("Cancel"),
            &gettext("Confirm"),
        )
        .await
        .is_ok()
    }
}
