pub use crate::globals::*;

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use arc_swap::ArcSwap;
use once_cell::sync::Lazy;

use crate::borg;
use crate::ui;

pub static SETTINGS: Lazy<ArcSwap<crate::config::Settings>> = Lazy::new(Default::default);
pub static BACKUP_COMMUNICATION: Lazy<ArcSwap<HashMap<String, borg::Communication>>> =
    Lazy::new(Default::default);
pub static ACTIVE_BACKUP_ID: Lazy<ArcSwap<Option<String>>> = Lazy::new(Default::default);
pub static INHIBIT_COOKIE: Lazy<ArcSwap<Option<u32>>> = Lazy::new(Default::default);

pub static ACTIVE_MOUNTS: Lazy<ArcSwap<HashSet<String>>> = Lazy::new(Default::default);
/// Is the app currently shutting down
pub static IS_SHUTDOWN: Lazy<ArcSwap<bool>> = Lazy::new(Default::default);

thread_local!(
    static MAIN_UI_STORE: Rc<ui::builder::Main> = Rc::new(ui::builder::Main::new());
    static GTK_APPLICATION: Rc<gtk::Application> = Rc::new({
        debug!("Setting up application with id '{}'", crate::app_id());
        gtk::ApplicationBuilder::new()
            .application_id(&crate::app_id())
            .build()
            .expect("Failed")
    });
);

pub fn main_ui() -> Rc<ui::builder::Main> {
    MAIN_UI_STORE.with(|x| x.clone())
}

pub fn gtk_app() -> Rc<gtk::Application> {
    GTK_APPLICATION.with(|x| x.clone())
}
