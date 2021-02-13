pub use crate::globals::*;

use crate::config::ConfigId;

use std::collections::{BTreeMap, HashSet};
use std::rc::Rc;

use arc_swap::ArcSwap;
use once_cell::sync::Lazy;

use crate::borg;
use crate::ui;

pub static SETTINGS: Lazy<ArcSwap<crate::config::Settings>> = Lazy::new(Default::default);
pub static BACKUP_COMMUNICATION: Lazy<ArcSwap<BTreeMap<ConfigId, borg::Communication>>> =
    Lazy::new(Default::default);
pub static ACTIVE_BACKUP_ID: Lazy<ArcSwap<Option<ConfigId>>> = Lazy::new(Default::default);
pub static INHIBIT_COOKIE: Lazy<ArcSwap<Option<u32>>> = Lazy::new(Default::default);

pub static ACTIVE_MOUNTS: Lazy<ArcSwap<HashSet<borg::RepoId>>> = Lazy::new(Default::default);
/// Is the app currently shutting down
pub static IS_SHUTDOWN: Lazy<ArcSwap<bool>> = Lazy::new(Default::default);

pub static LC_LOCALE: Lazy<num_format::Locale> = Lazy::new(|| {
    std::env::var("LC_NUMERIC")
        .ok()
        .as_deref()
        .and_then(|s| s.split('.').next())
        .map(|s| s.replace("_", "-"))
        .and_then(|name| num_format::Locale::from_name(&name).ok())
        .unwrap_or(num_format::Locale::fr)
});

thread_local!(
    static MAIN_UI_STORE: Rc<ui::builder::Main> = Rc::new(ui::builder::Main::new());
    static GTK_APPLICATION: Rc<gtk::Application> = Rc::new({
        debug!("Setting up application with id '{}'", crate::app_id());
        gtk::ApplicationBuilder::new()
            .application_id(&crate::app_id())
            .build()
    });
);

pub fn main_ui() -> Rc<ui::builder::Main> {
    MAIN_UI_STORE.with(|x| x.clone())
}

pub fn gtk_app() -> Rc<gtk::Application> {
    GTK_APPLICATION.with(|x| x.clone())
}
