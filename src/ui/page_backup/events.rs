use adw::prelude::*;

use crate::borg;
use crate::config;

use crate::ui;

use crate::ui::prelude::*;

use super::display;
use super::execution;

pub fn on_stack_changed() {
    if super::is_visible() {
        Handler::run(async { display::refresh() });
    }
}

/*
pub fn on_transition(stack: &adw::ViewStack) {
    if !stack.is_transition_running() && !super::is_visible() {
        for scrollable in &[main_ui().page_backup(), main_ui().page_archives()] {
            scrollable
                .vadjustment()
                .unwrap()
                .set_value(scrollable.vadjustment().unwrap().lower());
        }
    }
}
*/

pub async fn on_stop_backup_create() -> Result<()> {
    ui::utils::confirmation_dialog(
        &gettext("Abort running backup creation?"),
        &gettext("The backup will remain incomplete if aborted now."),
        &gettext("Continue"),
        &gettext("Abort"),
    )
    .await?;

    BACKUP_COMMUNICATION
        .load()
        .active()?
        .instruction
        .update(|inst| {
            *inst = borg::Instruction::Abort;
        });

    Ok(())
}

pub async fn on_backup_run() -> Result<()> {
    execution::start_backup(BACKUP_CONFIG.load().active()?.clone()).await
}

/// Returns a relative path for sub directories of home
fn rel_path(path: &std::path::Path) -> std::path::PathBuf {
    if let Ok(rel_path) = path.strip_prefix(glib::home_dir().as_path()) {
        rel_path.to_path_buf()
    } else {
        path.to_path_buf()
    }
}

pub async fn add_include() -> Result<()> {
    if let Some(path) =
        ui::utils::folder_chooser_dialog_path(&gettext("Include directory in backups")).await
    {
        BACKUP_CONFIG.update_result(|settings| {
            settings.active_mut()?.include.insert(rel_path(&path));
            Ok(())
        })?;
        crate::ui::write_config()?;
        display::refresh()?;
    }

    Ok(())
}

pub async fn add_exclude() -> Result<()> {
    if let Some(path) =
        ui::utils::folder_chooser_dialog_path(&gettext("Exclude directory from backup")).await
    {
        BACKUP_CONFIG.update_result(|settings| {
            settings
                .active_mut()?
                .exclude
                .insert(config::Pattern::PathPrefix(rel_path(&path)));
            Ok(())
        })?;
        crate::ui::write_config()?;
        display::refresh()?;
    }

    Ok(())
}

pub async fn on_include_home_changed() -> Result<()> {
    if main_ui().include_home().is_sensitive() {
        let change: bool = if main_ui().include_home().is_active() {
            true
        } else {
            confirm_remove_include(std::path::Path::new("Home")).await
        };

        BACKUP_CONFIG.update_result(|settings| {
            if !change {
                main_ui()
                    .include_home()
                    .set_active(!main_ui().include_home().is_active());
            } else if main_ui().include_home().is_active() {
                settings
                    .active_mut()?
                    .include
                    .insert(std::path::PathBuf::new());
            } else {
                settings
                    .active_mut()?
                    .include
                    .remove(&std::path::PathBuf::new());
            }
            Ok(())
        })?;

        if change {
            crate::ui::write_config()?;
            display::refresh()?;
        }
    } else {
        main_ui().include_home().set_sensitive(true);
    }

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
    ui::utils::confirmation_dialog(
        &gettextf(
            "No longer include “{}” in backups?",
            &[&path.to_string_lossy()],
        ),
        &gettext("All files contained in this folder will no longer be part of future backups."),
        &gettext("Cancel"),
        &gettext("Confirm"),
    )
    .await
    .is_ok()
}
