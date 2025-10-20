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
    LIB_USER
        .set(LibUser::Daemon)
        .expect("Could not set daemon mode for library.");
    init::init();
    gio_app().run();
}
