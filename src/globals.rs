use once_cell::sync::Lazy;
use once_cell::sync::OnceCell;

use crate::config;
use crate::prelude::*;

/// Session Bus
pub static ZBUS_SESSION: Lazy<zbus::Connection> = Lazy::new(|| {
    async_std::task::block_on(async {
        zbus::Connection::session()
            .await
            .expect("Failed to create ZBus session connection.")
    })
});

/// System Bus
pub static ZBUS_SYSTEM: Lazy<zbus::Connection> = Lazy::new(|| {
    async_std::task::block_on(async {
        zbus::Connection::system()
            .await
            .expect("Failed to create ZBus system connection.")
    })
});

pub static LIB_USER: OnceCell<LibUser> = OnceCell::new();

pub fn backup_config() -> std::sync::Arc<dyn LookupConfigId<Item = config::Backup>> {
    if matches!(LIB_USER.get(), Some(&LibUser::Daemon)) {
        Lazy::force(&crate::daemon::BACKUP_CONFIG).load().clone()
    } else {
        Lazy::force(&crate::ui::BACKUP_CONFIG).load().clone()
    }
}

pub fn backup_history() -> std::sync::Arc<dyn LookupConfigId<Item = config::history::History>> {
    if matches!(LIB_USER.get(), Some(&LibUser::Daemon)) {
        Lazy::force(&crate::daemon::BACKUP_HISTORY).load().clone()
    } else {
        Lazy::force(&crate::ui::BACKUP_HISTORY).load().clone()
    }
}

pub fn schedule_status() -> std::sync::Arc<dyn LookupConfigId<Item = config::Activity>> {
    if matches!(LIB_USER.get(), Some(&LibUser::Daemon)) {
        Lazy::force(&crate::daemon::SCHEDULE_STATUS).load().clone()
    } else {
        Lazy::force(&crate::ui::SCHEDULE_STATUS).load().clone()
    }
}

#[derive(Debug)]
pub enum LibUser {
    Daemon,
    Ui,
}
