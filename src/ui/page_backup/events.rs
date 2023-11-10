use std::ffi::OsStr;
use std::path::PathBuf;

use gtk::prelude::*;

use crate::borg;
use crate::ui;

use crate::ui::prelude::*;

use super::display;
use super::execution;

pub fn on_stack_changed() {
    if super::is_visible() {
        Handler::run(async { display::refresh() });
    }
}

pub async fn on_stop_backup_create() -> Result<()> {
    let operation = BORG_OPERATION.with(|op| Ok::<_, Error>(op.load().active()?.clone()))?;

    // Abort immediately if only reconnecting
    if !operation.aborting() && !matches!(operation.status(), borg::Run::Reconnecting(_)) {
        match operation.task_kind() {
            borg::task::Kind::Create => {
                ui::utils::confirmation_dialog(
                &gettext("Stop Running Backup?"),
                &gettext("The current backup state will be saved. You can continue your backup later by starting it again."),
                &gettext("Continue"),
                &gettext("Stop"),
            )
            .await?;
            }
            borg::task::Kind::Prune | borg::task::Kind::Delete => {
                ui::utils::confirmation_dialog(
                    &gettext("Abort Delete Operation?"),
                    &gettext("Archives are currently being deleted. Free space will not be reclaimed when aborting."),
                    &gettext("Continue"),
                    &gettext("Abort"),
                )
                .await?;
            }
            _ => {
                ui::utils::confirmation_dialog(
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

pub async fn on_backup_run() -> Result<()> {
    execution::start_backup(BACKUP_CONFIG.load().active()?.clone(), None).await
}

pub async fn on_backup_disk_eject() -> Result<()> {
    // Hide the button immediately to prevent accidental multiple triggers of the action
    // It will be shown again on error
    main_ui().backup_disk_eject_button().set_visible(false);

    let res = ui::utils::borg::unmount_backup_disk(BACKUP_CONFIG.load().active()?.clone()).await;
    super::display::refresh()?;
    res
}

pub async fn add_include() -> Result<()> {
    let chooser = gtk::FileDialog::builder()
        .initial_folder(&gio::File::for_path(glib::home_dir()))
        .title(gettext("Include Folder"))
        .accept_label(gettext("Select"))
        .modal(true)
        .build();

    let paths = ui::utils::paths_from_model(
        chooser
            .select_multiple_folders_future(Some(&main_ui().window()))
            .await
            .map_err(|err| match err.kind::<gtk::DialogError>() {
                Some(gtk::DialogError::Cancelled | gtk::DialogError::Dismissed) => {
                    Error::UserCanceled
                }
                _ => Message::short(err.to_string()).into(),
            })?,
    )?;

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
        BACKUP_CONFIG.try_update(|settings| {
            for path in &paths {
                settings
                    .active_mut()?
                    .include
                    .insert(ui::utils::rel_path(path));
            }
            Ok(())
        })?;

        display::refresh()?;
    }

    Ok(())
}

pub async fn add_exclude() -> Result<()> {
    ui::dialog_exclude::show();

    Ok(())
}

pub async fn on_remove_include(path: std::path::PathBuf) -> Result<()> {
    if confirm_remove_include(&path).await {
        BACKUP_CONFIG.try_update(|settings| {
            settings.active_mut()?.include.remove(&path);
            Ok(())
        })?;
        display::refresh()?;
    }

    Ok(())
}

async fn confirm_remove_include(path: &std::path::Path) -> bool {
    let path_string = if path == std::path::Path::new("") {
        gettext("Home")
    } else {
        path.display().to_string()
    };

    ui::utils::confirmation_dialog(
        &gettextf("No longer include “{}” in backups?", &[&path_string]),
        &gettext("All files contained in this folder will no longer be part of future backups."),
        &gettext("Cancel"),
        &gettext("Confirm"),
    )
    .await
    .is_ok()
}
