use gtk::prelude::*;

use crate::config;
use crate::ui;
use crate::ui::prelude::*;

pub async fn show() -> Result<()> {
    let storage = ui::builder::DialogStorage::new();

    storage
        .dialog()
        .set_transient_for(Some(&main_ui().window()));

    let backup = SETTINGS.load().backups.get_active()?.clone();
    match &backup.repo {
        config::BackupRepo::Local(repo) => {
            storage
                .volume()
                .set_text(&repo.mount_name.clone().unwrap_or_default());
            storage
                .device()
                .set_text(&repo.drive_name.clone().unwrap_or_default());
            storage.path().set_text(&repo.path().to_string_lossy());
            storage.disk().show();

            if let Some((fs_size, fs_free)) =
                ui::utils::fs_usage(&gio::File::new_for_path(&repo.path()))
            {
                storage
                    .fs_size()
                    .set_text(&glib::format_size(fs_size).unwrap());
                storage
                    .fs_free()
                    .set_text(&glib::format_size(fs_free).unwrap());
                storage
                    .fs_usage()
                    .set_value(1.0 - fs_free as f64 / fs_size as f64);
                storage.fs().show();
            }
        }
        repo @ config::BackupRepo::Remote { .. } => {
            storage.uri().set_text(&repo.to_string());
            storage.remote().show();
        }
    }

    storage.dialog().run_future().await;
    storage.dialog().close();

    Ok(())
}
