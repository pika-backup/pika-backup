use crate::borg::{self, prelude::*};
use crate::config;
use crate::ui;
use crate::ui::globals::*;
use crate::ui::prelude::*;

use gio::prelude::*;

use arc_swap::ArcSwap;
use once_cell::sync::Lazy;

static WAITING_CONFIGS: Lazy<ArcSwap<u64>> = Lazy::new(Default::default);

pub fn run() {
    if !SETTINGS
        .load()
        .backups
        .values()
        .any(|config| config.config_version < crate::CONFIG_VERSION)
    {
        return;
    }

    for config in SETTINGS.load().backups.values() {
        if config.config_version < crate::CONFIG_VERSION {
            ui::dialog_device_missing::main(
                config.clone(),
                &gettext("Updating configuration for new version"),
                enclose!((config) move || {
                    WAITING_CONFIGS.update(|value| {
                        *value += 1;
                    });
                    ui::page_pending::show(&gettext("Updating configuration for new version"));
                    glib::timeout_add_local(500, move || {
                        trace!("Configs waiting {}", WAITING_CONFIGS.load());
                        if WAITING_CONFIGS.get() < 1 {
                            ui::page_pending::back();
                            Continue(false)
                        } else {
                            Continue(true)
                        }
                    });

                    Handler::run(enclose!((config) async move {
                    let result =
                    ui::utils::Async::borg_spawn(
                        "refresh_repo_config",
                        borg::Borg::new(config.clone()),
                        |borg| borg.peek(),
                        )
                    .await.err_to_msg(gettext("Failed to retrive backup information."))?;

                    update_config(config.id.clone(), result)

                    }));
                }),
            );
        }
    }
}

fn update_config(id: ConfigId, list: borg::List) -> Result<()> {
    trace!("Got config update result");

    SETTINGS.update(move |settings| {
        if let Some(config) = settings.backups.get_mut(&id) {
            let icon_symbolic = match &config.repo {
                config::BackupRepo::Local { path, .. } => gio::File::new_for_path(path)
                    .find_enclosing_mount(Some(&gio::Cancellable::new()))
                    .ok()
                    .and_then(|m| m.get_symbolic_icon()),

                _ => None,
            };

            config.update_version_0(list.clone(), icon_symbolic);
        }
    });

    trace!("Finished this config update");
    ui::write_config()?;

    WAITING_CONFIGS.update(|value| {
        *value -= 1;
    });

    Ok(())
}
