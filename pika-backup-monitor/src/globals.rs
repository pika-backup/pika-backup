use std::collections::HashMap;
use std::rc::Rc;
use std::sync::LazyLock;

use arc_swap::ArcSwap;
use common::config;
pub use common::globals::*;

pub static BACKUP_CONFIG: LazyLock<ArcSwap<config::Backups>> = LazyLock::new(Default::default);
pub static BACKUP_HISTORY: LazyLock<ArcSwap<config::Histories>> = LazyLock::new(Default::default);

pub static SCHEDULE_STATUS: LazyLock<ArcSwap<config::Writeable<config::ScheduleStatus>>> =
    LazyLock::new(Default::default);

/// Last reminded about not meeting criteria
pub static LAST_REMINDED: LazyLock<ArcSwap<HashMap<config::ConfigId, std::time::Instant>>> =
    LazyLock::new(Default::default);

thread_local!(
    static GIO_APPLICATION: Rc<gio::Application> = Rc::new({
        tracing::debug!("Creating gio::Application {:?}", common::DAEMON_APP_ID);
        gio::Application::new(
            Some(common::DAEMON_APP_ID),
            gio::ApplicationFlags::IS_SERVICE | gio::ApplicationFlags::ALLOW_REPLACEMENT,
        )
    });
);

pub fn gio_app() -> Rc<gio::Application> {
    GIO_APPLICATION.with(Clone::clone)
}
