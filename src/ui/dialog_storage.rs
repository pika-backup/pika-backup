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
                .set_text(&repo.mount_name.clone().unwrap_or_default());
            storage
                .device()
                .set_text(&repo.drive_name.clone().unwrap_or_default());
            storage.path().set_text(&repo.path().to_string_lossy());
            storage.disk().show();
        }
        repo @ config::Repository::Remote { .. } => {
            storage.uri().set_text(&repo.to_string());

            storage.remote().show();
        }
    }

    if let Some(df) = ui::utils::df::cached_or_lookup(&backup).await {
        show_df(&df, &storage);
    }

    storage.dialog().run_future().await;
    storage.dialog().close();

    Ok(())
}

fn show_df(df: &ui::utils::df::Space, ui: &ui::builder::DialogStorage) {
    ui.fs_size().set_text(&glib::format_size(df.size));
    ui.fs_free().set_text(&glib::format_size(df.avail));
    ui.fs_usage()
        .set_value(1.0 - df.avail as f64 / df.size as f64);
    ui.fs().show();
}
