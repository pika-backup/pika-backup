use std::sync::LazyLock;
use std::sync::OnceLock;

use crate::config;
use crate::prelude::*;

const CLOCK_INTERFACE: &str = "org.gnome.desktop.interface";
const CLOCK_KEY: &str = "clock-format";

pub static LIB_USER: OnceLock<LibUser> = OnceLock::new();

pub static APP_IS_SANDBOXED: LazyLock<bool> =
    LazyLock::new(|| async_std::task::block_on(ashpd::is_sandboxed()));

pub static CLOCK_IS_24H: LazyLock<bool> = LazyLock::new(|| {
    async_std::task::block_on(async {
        let proxy = ashpd::desktop::settings::Settings::new().await?;
        proxy.read::<String>(CLOCK_INTERFACE, CLOCK_KEY).await
    })
    .map(|s| s == "24h")
    .inspect_err(|e| warn!("Retrieving '{}' setting failed: {}", CLOCK_KEY, e))
    .unwrap_or_default()
});

pub static MEMORY_PASSWORD_STORE: LazyLock<
    std::sync::Arc<crate::utils::password::MemoryPasswordStore>,
> = LazyLock::new(Default::default);

pub fn backup_config() -> std::sync::Arc<dyn LookupConfigId<Item = config::Backup>> {
    if matches!(LIB_USER.get(), Some(&LibUser::Daemon)) {
        LazyLock::force(&crate::daemon::BACKUP_CONFIG)
            .load()
            .clone()
    } else {
        LazyLock::force(&crate::ui::BACKUP_CONFIG).load().clone()
    }
}

pub fn backup_history() -> std::sync::Arc<dyn LookupConfigId<Item = config::history::History>> {
    if matches!(LIB_USER.get(), Some(&LibUser::Daemon)) {
        LazyLock::force(&crate::daemon::BACKUP_HISTORY)
            .load()
            .clone()
    } else {
        LazyLock::force(&crate::ui::BACKUP_HISTORY).load().clone()
    }
}

pub fn schedule_status() -> std::sync::Arc<dyn LookupConfigId<Item = config::Activity>> {
    if matches!(LIB_USER.get(), Some(&LibUser::Daemon)) {
        LazyLock::force(&crate::daemon::SCHEDULE_STATUS)
            .load()
            .clone()
    } else {
        LazyLock::force(&crate::ui::SCHEDULE_STATUS).load().clone()
    }
}

#[derive(Debug)]
pub enum LibUser {
    Daemon,
    Ui,
}
