#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate quick_error;
#[macro_use]
extern crate enclose;
#[macro_use]
pub mod prelude;
#[macro_use]
extern crate async_trait;

static BORG_DELAY_RECONNECT: std::time::Duration = std::time::Duration::from_secs(60);
static BORG_MAX_RECONNECT: u16 = 30;
static BORG_LOCK_WAIT_RECONNECT: std::time::Duration = std::time::Duration::from_secs(60 * 7);

const REPO_MOUNT_DIR: &str = ".mnt/borg";

// require borg 1.1
const BORG_MIN_MAJOR: u32 = 1;
const BORG_MIN_MINOR: u32 = 1;

const NON_JOURNALING_FILESYSTEMS: &[&str] = &["exfat", "ext2", "vfat"];

const DEFAULT_LOCALEDIR: &str = "/usr/share/locale";

macro_rules! data_dir {
    () => {
        concat!(env!("CARGO_MANIFEST_DIR"), "/data")
    };
}

macro_rules! application_id {
    () => {
        include_str!(concat!(data_dir!(), "/APPLICATION_ID"))
    };
}

pub fn app_id() -> String {
    format!(
        "{}{}",
        application_id!(),
        option_env!("APPLICATION_ID_SUFFIX").unwrap_or_default()
    )
}

pub fn daemon_app_id() -> String {
    format!("{}.Daemon", app_id())
}

pub mod action;
pub mod borg;
pub mod config;
pub mod daemon;
pub mod globals;
pub mod ui;
pub mod utils;
