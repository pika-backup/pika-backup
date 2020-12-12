use crate::borg::{self, prelude::*};
use crate::shared;
use crate::ui;
use crate::ui::globals::*;
use crate::ui::prelude::*;

use gio::prelude::*;

use std::sync::Arc;

pub fn run() {
    ui::page_pending::show(&gettext("Updating configuration for new version"));
    let finished = Arc::new(());
    for config in SETTINGS.load().backups.values() {
        ui::dialog_device_missing::main(
            config.clone(),
            enclose!((config, finished) move || {
            ui::utils::Async::borg(
                "refresh_repo_config",
                borg::Borg::new(config.clone()),
                |borg| borg.peek(),
                 enclose!((config, finished) move |result| x(config.id.clone(), finished.clone(), result)),
            );
            }),
        );
    }

    glib::timeout_add_local(500, move || {
        trace!("Strong reference count {}", Arc::strong_count(&finished));
        if Arc::strong_count(&finished) <= 1 {
            ui::write_config();
            ui::page_pending::back();
            Continue(false)
        } else {
            Continue(true)
        }
    });
}

fn x(id: String, _finished: Arc<()>, result: Result<borg::List, shared::BorgErr>) {
    trace!("Got config update result");

    match result {
        Ok(list) => {
            SETTINGS.update(move |settings| {
                if let Some(config) = settings.backups.get_mut(&id) {
                    let icon_symbolic = match &config.repo {
                        shared::BackupRepo::Local { path, .. } => gio::File::new_for_path(path)
                            .find_enclosing_mount(Some(&gio::Cancellable::new()))
                            .ok()
                            .and_then(|m| m.get_symbolic_icon()),

                        _ => None,
                    };

                    config.update_version_0(list.clone(), icon_symbolic);
                }
            });
        }

        Err(err) => {
            ui::utils::show_error(gettext("Failed to update config"), err);
        }
    }
}
