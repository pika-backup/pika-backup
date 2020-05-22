use gtk::prelude::*;

use crate::shared;
use crate::ui;
use crate::ui::globals::*;
use crate::ui::prelude::*;

pub fn show() {
    let storage = ui::builder::Storage::new();

    storage
        .dialog()
        .set_transient_for(Some(&main_ui().window()));

    let backup = SETTINGS.load().backups.get_active().unwrap().clone();
    match &backup.repo {
        shared::BackupRepo::Local {
            ref label,
            ref device,
            ref path,
            ..
        } => {
            storage
                .volume()
                .set_text(&label.clone().unwrap_or_default());
            storage
                .device()
                .set_text(&device.clone().unwrap_or_default());
            storage.path().set_text(&path.to_string_lossy());
            storage.disk().show();

            if let Some((fs_size, fs_free)) = ui::utils::fs_usage(&gio::File::new_for_path(&path)) {
                storage.fs_size().set_text(&ui::utils::hsized(fs_size, 0));
                storage.fs_free().set_text(&ui::utils::hsized(fs_free, 0));
                storage
                    .fs_usage()
                    .set_value(1.0 - fs_free as f64 / fs_size as f64);
                storage.fs().show();
            }
        }
        repo @ shared::BackupRepo::Remote { .. } => {
            storage.uri().set_text(&repo.to_string());
            storage.remote().show();
        }
    }

    storage.dialog().run();
    storage.dialog().close();
}
