use gio::prelude::*;

use crate::config;
use crate::config::TrackChanges;
use crate::daemon;
use crate::daemon::prelude::*;

pub fn init() {
    gio_app().connect_startup(on_startup);
}

fn on_startup(_app: &gio::Application) {
    gio_app().hold();

    config::Histories::update_on_change(&BACKUP_HISTORY).handle("Initial config load failed");
    config::Backups::update_on_change(&BACKUP_CONFIG).handle("Initial config load failed");

    daemon::connect::init::init();
    daemon::schedule::init::init();
}
