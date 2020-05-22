#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
#[macro_use]
extern crate quick_error;
#[macro_use]
extern crate matches;
#[macro_use]
extern crate enclose;

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

static APPLICATION_ID: &str = application_id!();
static APPLICATION_NAME: &str = include_str!(concat!(data_dir!(), "/APPLICATION_NAME"));

static BORG_DELAY_RECONNECT: std::time::Duration = std::time::Duration::from_secs(60);
static BORG_LOCK_WAIT_RECONNECT: std::time::Duration = std::time::Duration::from_secs(60 * 7);

const REPO_MOUNT_DIR: &str = ".mnt/borg";

pub mod borg;
pub mod globals;
pub mod shared;
pub mod ui;
