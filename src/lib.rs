#![allow(clippy::new_without_default)]
//requires rust 1.63
//#![allow(clippy::derive_partial_eq_without_eq)]

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate quick_error;
#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate enclose;
#[macro_use]
mod prelude;

use default_env::default_env;

const REPO_MOUNT_DIR: &str = ".mnt/borg";

const NON_JOURNALING_FILESYSTEMS: &[&str] = &["exfat", "ext2", "vfat"];

const LOCALEDIR: &str = default_env!("LOCALEDIR", "/usr/share/locale");

const APP_ID_WITHOUT_SUFFIX: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data",
    "/APPLICATION_ID"
));
const APP_ID_SUFFIX: &str = default_env!("APPLICATION_ID_SUFFIX", "");

const APP_ID: &str = const_str::concat!(APP_ID_WITHOUT_SUFFIX, APP_ID_SUFFIX);
const DBUS_API_NAME: &str = const_str::concat!(APP_ID, ".Api");
const DBUS_API_PATH: &str = const_str::concat!("/", const_str::replace!(APP_ID, ".", "/"));

const DAEMON_APP_ID: &str = const_str::concat!(APP_ID, ".Montior");
const DAEMON_BINARY: &str = concat!(env!("CARGO_PKG_NAME"), "-monitor");

mod action;
pub mod borg;
pub mod config;
pub mod daemon;
mod globals;
mod schedule;
pub mod ui;
mod utils;
