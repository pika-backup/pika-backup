use std::collections::{BTreeMap, HashSet};
use std::rc::Rc;
use std::sync::{LazyLock, OnceLock};

use arc_swap::ArcSwap;

use super::app::App;
use crate::config::ConfigId;
use crate::{borg, config, ui};

pub static BACKUP_CONFIG: LazyLock<ArcSwap<config::Writeable<config::Backups>>> =
    LazyLock::new(Default::default);
pub static BACKUP_HISTORY: LazyLock<ArcSwap<config::Writeable<config::Histories>>> =
    LazyLock::new(Default::default);

pub static SCHEDULE_STATUS: LazyLock<ArcSwap<config::ScheduleStatus>> =
    LazyLock::new(Default::default);

pub static ACTIVE_BACKUP_ID: LazyLock<ArcSwap<Option<ConfigId>>> = LazyLock::new(Default::default);

pub static ACTIVE_MOUNTS: LazyLock<ArcSwap<HashSet<borg::RepoId>>> =
    LazyLock::new(Default::default);

pub static BORG_VERSION: OnceLock<String> = OnceLock::new();

pub static REPO_CACHE: LazyLock<ArcSwap<BTreeMap<borg::RepoId, ui::utils::repo_cache::RepoCache>>> =
    LazyLock::new(Default::default);

pub static LC_LOCALE: LazyLock<num_format::Locale> = LazyLock::new(|| {
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
