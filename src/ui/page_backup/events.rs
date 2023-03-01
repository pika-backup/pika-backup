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

    if operation.aborting() {
        ui::utils::confirmation_dialog(
            &gettext("Abort Saving Backup State?"),
            &gettext(
                "The current backup state is in the process of being saved. The backup can be continued later without saving the state. Some data might have to be copied again.",
            ),
            &gettext("Continue"),
            &gettext("Abort"),
        )
        .await?;
    } else {
        ui::utils::confirmation_dialog(
            &gettext("Stop Running Backup?"),
            &gettext("The current backup state will be saved. You can continue your backup later by starting it again."),
            &gettext("Continue"),
            &gettext("Stop"),
        )
        .await?;
    }

    operation.set_instruction(borg::Instruction::Abort(borg::Abort::User));

    Ok(())
}

pub async fn on_backup_run() -> Result<()> {
    execution::start_backup(BACKUP_CONFIG.load().active()?.clone(), None).await
}

pub async fn add_include() -> Result<()> {
    let chooser = gtk::FileChooserNative::builder()
        .action(gtk::FileChooserAction::SelectFolder)
        .select_multiple(true)
        .title(gettext("Include Folder"))
        .accept_label(gettext("Select"))
        .modal(true)
        .transient_for(&main_ui().window())
        .build();

    let paths = ui::utils::paths(chooser).await?;

    BACKUP_CONFIG.update_result(|settings| {
        for path in &paths {
            settings
                .active_mut()?
                .include
                .insert(ui::utils::rel_path(path));
        }
        Ok(())
    })?;

    crate::ui::write_config()?;
    display::refresh()?;

    Ok(())
}

pub async fn add_exclude() -> Result<()> {
    ui::dialog_exclude::show();

    Ok(())
}

pub async fn on_remove_include(path: std::path::PathBuf) -> Result<()> {
    if confirm_remove_include(&path).await {
        BACKUP_CONFIG.update_result(|settings| {
            settings.active_mut()?.include.remove(&path);
            Ok(())
        })?;
        crate::ui::write_config()?;
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
