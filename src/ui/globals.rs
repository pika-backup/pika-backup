pub use crate::globals::*;
pub use glib::prelude::*;

use crate::config;
use crate::config::ConfigId;

use std::collections::{BTreeMap, HashSet};
use std::rc::Rc;

use arc_swap::ArcSwap;
use once_cell::sync::Lazy;

use crate::borg;
use crate::ui;

pub static BACKUP_CONFIG: Lazy<ArcSwap<config::Writeable<config::Backups>>> =
    Lazy::new(Default::default);
pub static BACKUP_HISTORY: Lazy<ArcSwap<config::Writeable<config::Histories>>> =
    Lazy::new(Default::default);

pub static BACKUP_COMMUNICATION: Lazy<ArcSwap<BTreeMap<ConfigId, borg::Communication>>> =
    Lazy::new(Default::default);
pub static BACKUP_LOCK: Lazy<ArcSwap<BTreeMap<borg::RepoId, String>>> = Lazy::new(Default::default);
pub static ACTIVE_BACKUP_ID: Lazy<ArcSwap<Option<ConfigId>>> = Lazy::new(Default::default);
pub static INHIBIT_COOKIE: Lazy<ArcSwap<Option<u32>>> = Lazy::new(Default::default);

pub static ACTIVE_MOUNTS: Lazy<ArcSwap<HashSet<borg::RepoId>>> = Lazy::new(Default::default);
/// Is the app currently shutting down
pub static IS_SHUTDOWN: Lazy<ArcSwap<bool>> = Lazy::new(Default::default);

pub static REPO_CACHE: Lazy<ArcSwap<BTreeMap<borg::RepoId, ui::utils::repo_cache::RepoCache>>> =
    Lazy::new(Default::default);

pub static DAEMON_RUNNING: Lazy<ArcSwap<bool>> = Lazy::new(Default::default);

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
    static MAIN_UI_STORE: Rc<ui::builder::AppWindow> = Rc::new({
        ui::widgets::init();
        ui::builder::AppWindow::new()
    });

    static GTK_APPLICATION: Rc<gtk::Application> = Rc::new({
        debug!("Setting up application with id '{}'", crate::app_id());
        gtk::Application::builder()
            .application_id(&crate::app_id())
            .build()
    });
);

pub fn main_ui() -> Rc<ui::builder::AppWindow> {
    MAIN_UI_STORE.with(|x| x.clone())
}

pub fn gtk_app() -> Rc<gtk::Application> {
    GTK_APPLICATION.with(|x| x.clone())
}
