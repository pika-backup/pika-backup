use crate::config;
use crate::config::ConfigId;

use std::collections::{BTreeMap, HashSet};
use std::rc::Rc;
use std::sync::OnceLock;

use arc_swap::ArcSwap;
use once_cell::sync::Lazy;

use crate::borg;
use crate::ui;

use super::app::App;

pub static BACKUP_CONFIG: Lazy<ArcSwap<config::Writeable<config::Backups>>> =
    Lazy::new(Default::default);
pub static BACKUP_HISTORY: Lazy<ArcSwap<config::Writeable<config::Histories>>> =
    Lazy::new(Default::default);

pub static SCHEDULE_STATUS: Lazy<ArcSwap<config::ScheduleStatus>> = Lazy::new(Default::default);

pub static ACTIVE_BACKUP_ID: Lazy<ArcSwap<Option<ConfigId>>> = Lazy::new(Default::default);

pub static ACTIVE_MOUNTS: Lazy<ArcSwap<HashSet<borg::RepoId>>> = Lazy::new(Default::default);

pub static BORG_VERSION: OnceLock<String> = OnceLock::new();

pub static REPO_CACHE: Lazy<ArcSwap<BTreeMap<borg::RepoId, ui::utils::repo_cache::RepoCache>>> =
    Lazy::new(Default::default);

pub static LC_LOCALE: Lazy<num_format::Locale> = Lazy::new(|| {
    std::env::var("LC_NUMERIC")
        .ok()
        .as_deref()
        .and_then(|s| s.split('.').next())
        .map(|s| s.replace('_', "-"))
        .and_then(|name| num_format::Locale::from_name(name).ok())
        .unwrap_or(num_format::Locale::fr)
});

thread_local!(
    pub static BORG_OPERATION: ArcSwap<BTreeMap<ConfigId, Rc<dyn ui::operation::OperationExt>>> =
        Default::default();

    pub static STATUS_TRACKING: Rc<ui::status::StatusTracking> =
        ui::status::StatusTracking::new_rc();
);

pub fn main_ui() -> ui::widget::AppWindow {
    App::default().main_window()
}

pub fn adw_app() -> ui::App {
    App::default()
}

pub fn status_tracking() -> Rc<ui::status::StatusTracking> {
    STATUS_TRACKING.with(|x| x.clone())
}
