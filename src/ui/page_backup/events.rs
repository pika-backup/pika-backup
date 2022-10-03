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
    ui::utils::confirmation_dialog(
        &gettext("Abort running backup creation?"),
        &gettext("The backup will remain incomplete if aborted now."),
        &gettext("Continue"),
        &gettext("Abort"),
    )
    .await?;

    BORG_OPERATION.with(|op| {
        op.load()
            .active()?
            .set_instruction(borg::Instruction::Abort(borg::Abort::User));
        Ok::<_, Error>(())
    })?;

    Ok(())
}

pub async fn on_backup_run() -> Result<()> {
    execution::start_backup(BACKUP_CONFIG.load().active()?.clone(), None).await
}

pub async fn add_include() -> Result<()> {
    ui::dialog_include::show();

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
