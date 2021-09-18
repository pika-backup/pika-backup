mod connect;
mod error;
mod globals;
mod init;
mod prelude;
mod schedule;
mod utils;

use gio::prelude::*;
use prelude::*;

pub fn main() {
    init::init();
    gio_app().run();
}
