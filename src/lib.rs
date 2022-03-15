#![allow(clippy::new_without_default)]

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate quick_error;
#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate enclose;
#[macro_use]
pub mod prelude;

const REPO_MOUNT_DIR: &str = ".mnt/borg";

const NON_JOURNALING_FILESYSTEMS: &[&str] = &["exfat", "ext2", "vfat"];

macro_rules! data_dir {
    () => {
        concat!(env!("CARGO_MANIFEST_DIR"), "/data")
    };
}

const UNPREFIXED_APP_ID: &str = include_str!(concat!(data_dir!(), "/APPLICATION_ID"));

fn app_id() -> String {
    format!(
        "{}{}",
        UNPREFIXED_APP_ID,
        option_env!("APPLICATION_ID_SUFFIX").unwrap_or_default()
    )
}

fn dbus_api_name() -> String {
    format!(
        "{}{}.Api",
        UNPREFIXED_APP_ID,
        option_env!("APPLICATION_ID_SUFFIX").unwrap_or_default()
    )
}

fn dbus_api_path() -> String {
    format!("/{}", app_id().replace('.', "/"))
}

fn daemon_app_id() -> String {
    format!(
        "{}.Daemon{}",
        UNPREFIXED_APP_ID,
        option_env!("APPLICATION_ID_SUFFIX").unwrap_or_default()
    )
}

mod action;
pub mod borg;
pub mod config;
pub mod daemon;
mod globals;
mod schedule;
pub mod ui;
mod utils;
