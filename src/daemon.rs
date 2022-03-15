mod connect;
mod dbus;
mod error;
mod globals;
mod init;
mod prelude;
mod schedule;

pub use globals::{BACKUP_CONFIG, SCHEDULE_STATUS};

use gio::prelude::*;
use prelude::*;

pub fn main() {
    init::init();
    gio_app().run();
}
