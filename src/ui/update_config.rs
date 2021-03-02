use crate::borg::{self, prelude::*};
use crate::config;
use crate::ui;
use crate::ui::prelude::*;

use gio::prelude::*;

use arc_swap::ArcSwap;
use once_cell::sync::Lazy;

static WAITING_CONFIGS: Lazy<ArcSwap<u64>> = Lazy::new(Default::default);

pub async fn run() -> Result<()> {
    if !BACKUP_CONFIG
        .load()
        .iter()
        .any(|config| config.config_version < crate::config::VERSION)
    {
        return Ok(());
    }

    for config in BACKUP_CONFIG.load().iter() {
        if config.config_version < crate::config::VERSION {
            let config = ui::dialog_device_missing::updated_config(
                config.clone(),
                &gettext("Updating configuration for new version"),
            )
            .await?;

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

            let result = ui::utils::borg::spawn(
                "refresh_repo_config",
                borg::Borg::new(config.clone()),
                |borg| borg.peek(),
            )
            .await
            .err_to_msg(gettext("Failed to retrieve backup information."))?;

            update_config(config.id.clone(), result)?;
        }
    }

    Ok(())
}

fn update_config(id: ConfigId, list: borg::List) -> Result<()> {
    trace!("Got config update result");

    BACKUP_CONFIG.update(move |settings| {
        if let Ok(config) = settings.get_mut_result(&id) {
            let icon_symbolic = match &config.repo {
                config::Repository::Local(local) => gio::File::new_for_path(local.path())
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
