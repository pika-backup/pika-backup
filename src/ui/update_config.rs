use crate::config;
use crate::ui;
use crate::ui::prelude::*;

use gio::prelude::*;

use arc_swap::ArcSwap;
use once_cell::sync::Lazy;

static WAITING_CONFIGS: Lazy<ArcSwap<u64>> = Lazy::new(Default::default);

pub fn run() {
    if !BACKUP_CONFIG
        .load()
        .iter()
        .any(|config| config.config_version < crate::config::VERSION)
    {
        return;
    }
    ui::page_pending::show(&gettext("Updating configuration for new version"));

    for config in BACKUP_CONFIG.load().iter().cloned() {
        if config.config_version < crate::config::VERSION {
            Handler::run(async move {
                WAITING_CONFIGS.update(|value| {
                    *value += 1;
                });

                let result = update_config(&config).await;

                WAITING_CONFIGS.update(|value| {
                    *value -= 1;
                });

                if WAITING_CONFIGS.get() < 1 {
                    ui::page_pending::back();
                }

                result
            });
        }
    }
}

async fn update_config(config: &config::Backup) -> Result<()> {
    let list = ui::utils::borg::exec(
        gettext("Updating configuration for new version"),
        config.clone(),
        |borg| borg.peek(),
    )
    .await
    .into_message(gettext("Failed to retrieve backup information."))?;

    trace!("Got config update result");

    BACKUP_CONFIG.update(move |settings| {
        if let Ok(config) = settings.get_mut_result(&config.id) {
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

    Ok(())
}
