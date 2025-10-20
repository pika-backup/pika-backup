pub mod borg;
pub mod config;
pub mod globals;
pub mod prelude;
pub mod schedule;
pub mod utils;

use default_env::default_env;

pub const LOCALEDIR: &str = default_env!("LOCALEDIR", "/usr/share/locale");
pub const APP_ID_WITHOUT_SUFFIX: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../data",
    "/APPLICATION_ID"
));
pub const APP_ID_SUFFIX: &str = default_env!("APPLICATION_ID_SUFFIX", "");

pub const APP_ID: &str = const_str::concat!(APP_ID_WITHOUT_SUFFIX, APP_ID_SUFFIX);
pub const DBUS_API_NAME: &str = const_str::concat!(APP_ID, ".Api");
pub const DBUS_API_PATH: &str = const_str::concat!("/", const_str::replace!(APP_ID, ".", "/"));

pub const DAEMON_APP_ID: &str = const_str::concat!(APP_ID, ".Monitor");
pub const DAEMON_BINARY: &str = "pika-backup-monitor";
