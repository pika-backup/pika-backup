//! Daemon

mod action;
mod connect;
mod dbus;
mod error;
mod globals;
mod init;
mod prelude;
mod schedule;

pub(crate) use globals::{BACKUP_CONFIG, BACKUP_HISTORY, SCHEDULE_STATUS};

use gio::prelude::*;
use prelude::*;

pub fn main() {
    LIB_USER
        .set(LibUser::Daemon)
        .expect("Could not set daemon mode for library.");
    init::init();
    gio_app().run();
}
