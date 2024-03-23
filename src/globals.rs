use once_cell::sync::Lazy;
use once_cell::sync::OnceCell;

use crate::config;
use crate::prelude::*;

const CLOCK_INTERFACE: &str = "org.gnome.desktop.interface";
const CLOCK_KEY: &str = "clock-format";

pub static LIB_USER: OnceCell<LibUser> = OnceCell::new();

pub static APP_IS_SANDBOXED: Lazy<bool> =
    Lazy::new(|| async_std::task::block_on(ashpd::is_sandboxed()));

pub static CLOCK_IS_24H: Lazy<bool> = Lazy::new(|| {
    async_std::task::block_on(async {
        let proxy = ashpd::desktop::settings::Settings::new().await?;
        proxy.read::<String>(CLOCK_INTERFACE, CLOCK_KEY).await
    })
    .map(|s| s == "24h")
    .inspect_err(|e| warn!("Retrieving '{}' setting failed: {}", CLOCK_KEY, e))
    .unwrap_or_default()
});

pub static MEMORY_PASSWORD_STORE: Lazy<
    std::sync::Arc<crate::utils::password::MemoryPasswordStore>,
> = Lazy::new(Default::default);

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
