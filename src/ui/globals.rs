use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use arc_swap::ArcSwap;
use once_cell::sync::Lazy;

use crate::borg;
use crate::ui;

pub static SETTINGS: Lazy<ArcSwap<crate::shared::Settings>> = Lazy::new(Default::default);
pub static BACKUP_COMMUNICATION: Lazy<ArcSwap<HashMap<String, borg::Communication>>> =
    Lazy::new(Default::default);
pub static ACTIVE_BACKUP_ID: Lazy<ArcSwap<Option<String>>> = Lazy::new(Default::default);
pub static INHIBIT_COOKIE: Lazy<ArcSwap<Option<u32>>> = Lazy::new(Default::default);

pub static ACTIVE_MOUNTS: Lazy<ArcSwap<HashSet<String>>> = Lazy::new(Default::default);
/// Is the app currently shutting down
pub static IS_SHUTDOWN: Lazy<ArcSwap<bool>> = Lazy::new(Default::default);

thread_local!(
    static MAIN_UI_STORE: Rc<ui::builder::Main> = Rc::new(ui::builder::Main::new());
    static GTK_APPLICATION: Rc<gtk::Application> = Rc::new(
        gtk::Application::new(
            Some(crate::APPLICATION_ID),
            gio::ApplicationFlags::FLAGS_NONE,
        )
        .expect("Failed to gtk::Application::new()"),
    );
);

pub fn main_ui() -> Rc<ui::builder::Main> {
    MAIN_UI_STORE.with(|x| x.clone())
}

pub fn gtk_app() -> Rc<gtk::Application> {
    GTK_APPLICATION.with(|x| x.clone())
}
