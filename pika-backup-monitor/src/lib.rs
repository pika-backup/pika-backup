//! Daemon

mod action;
mod connect;
mod dbus;
mod error;
mod globals;
mod init;
mod notification;
mod prelude;
mod schedule;

use gio::prelude::*;
use prelude::*;

pub fn main() {
    common::utils::init_logging("pika-backup-monitor");
    init::init();
    gio_app().run();
}
