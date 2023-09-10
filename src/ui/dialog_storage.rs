use adw::traits::ActionRowExt;
use gtk::prelude::*;

use crate::config;
use crate::ui;
use crate::ui::prelude::*;

pub async fn show() -> Result<()> {
    let storage = ui::builder::DialogStorage::new();

    storage
        .dialog()
        .set_transient_for(Some(&main_ui().window()));

    let backup = BACKUP_CONFIG.load().active()?.clone();
    match &backup.repo {
        config::Repository::Local(repo) => {
            storage
                .volume()
                .set_subtitle(&repo.mount_name.clone().unwrap_or_default());
            storage
                .device()
                .set_subtitle(&repo.drive_name.clone().unwrap_or_default());
            storage.path().set_subtitle(&repo.path().to_string_lossy());
            storage.disk().set_visible(true);
        }
        config::Repository::Remote { .. } => {
            storage.uri().set_subtitle(&backup.repo.to_string());

            storage.remote().set_visible(true);
        }
    }

    if let Some(df) = ui::utils::df::cached_or_lookup(&backup).await {
        show_df(&df, &storage);
    }

    storage.dialog().set_visible(true);

    Ok(())
}

fn show_df(df: &ui::utils::df::Space, ui: &ui::builder::DialogStorage) {
    ui.fs_size().set_subtitle(&glib::format_size(df.size));
    ui.fs_free().set_subtitle(&glib::format_size(df.avail));
    ui.fs_usage()
        .set_value(1.0 - df.avail as f64 / df.size as f64);
    ui.fs().set_visible(true);
}
