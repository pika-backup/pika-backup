use arc_swap::ArcSwap;
use once_cell::sync::Lazy;

pub static BACKUP_CONFIG: Lazy<ArcSwap<crate::config::Backups>> = Lazy::new(Default::default);

pub static HOME_DIR: Lazy<std::path::PathBuf> =
    Lazy::new(|| glib::get_home_dir().expect("Could not determine home directory."));

pub static CONFIG_DIR: Lazy<std::path::PathBuf> =
    Lazy::new(|| glib::get_user_config_dir().expect("Could not determine config directory."));

pub fn init() {
    Lazy::force(&HOME_DIR);
    Lazy::force(&CONFIG_DIR);
}
