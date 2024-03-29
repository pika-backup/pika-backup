use crate::config;
use crate::config::Loadable;
use crate::ui::prelude::*;
use config::ArcSwapWriteable;

fn load_config_e() -> std::io::Result<()> {
    if glib::user_config_dir()
        .join(env!("CARGO_PKG_NAME"))
        .join("config.json")
        .is_file()
        && !glib::user_config_dir()
            .join(env!("CARGO_PKG_NAME"))
            .join("backup.json")
            .is_file()
    {
        std::fs::rename(
            glib::user_config_dir()
                .join(env!("CARGO_PKG_NAME"))
                .join("config.json"),
            glib::user_config_dir()
                .join(env!("CARGO_PKG_NAME"))
                .join("backup.json"),
        )?;
    }

    BACKUP_CONFIG.swap(Arc::new(config::Writeable::from_file()?));
    BACKUP_CONFIG.update_no_commit(|backups| {
        for config in backups.iter_mut() {
            if config.config_version.0 < config::VERSION {
                config.config_version = Default::default();
            }
        }
    });
    // potentially write generated default value
    BACKUP_CONFIG.write_file()?;

    BACKUP_HISTORY.swap(Arc::new(config::Histories::from_file_ui()?));
    // potentially write internal error status
    BACKUP_HISTORY.write_file()?;

    Ok(())
}

pub fn load_config() {
    let res = load_config_e().err_to_msg(gettext("Could not load configuration file."));
    if let Err(err) = res {
        glib::MainContext::default().spawn_local(async move { err.show().await });
    }
}
