pub use crate::config::ConfigId;
pub use crate::globals::*;
pub use crate::utils::LookupConfigId;
pub use std::rc::Rc;
pub use std::sync::Arc;

use crate::config;

use arc_swap::ArcSwap;

#[allow(clippy::implicit_hasher)]
impl<T> LookupConfigId<T> for std::collections::BTreeMap<ConfigId, T> {
    fn get_mut_result(&mut self, key: &ConfigId) -> Result<&mut T, config::error::BackupNotFound> {
        self.get_mut(&key)
            .ok_or_else(|| config::error::BackupNotFound::new(key.clone()))
    }

    fn get_result(&self, key: &ConfigId) -> Result<&T, config::error::BackupNotFound> {
        self.get(key)
            .ok_or_else(|| config::error::BackupNotFound::new(key.clone()))
    }
}

pub trait ArcSwapExt<T> {
    fn update<F: Fn(&mut T)>(&self, updater: F);
    fn get(&self) -> T;
}

impl<T> ArcSwapExt<T> for ArcSwap<T>
where
    T: Clone,
{
    fn update<F: Fn(&mut T)>(&self, updater: F) {
        self.rcu(|current| {
            let mut new = T::clone(current);
            updater(&mut new);
            new
        });
    }

    fn get(&self) -> T {
        T::clone(&self.load_full())
    }
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)+) => (
        glib::g_log!(module_path!(), glib::LogLevel::Critical, $($arg)+)
    )
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)+) => (
        glib::g_log!(module_path!(), glib::LogLevel::Warning, $($arg)+)
    )
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)+) => (
        glib::g_log!(module_path!(), glib::LogLevel::Info, $($arg)+)
    )
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)+) => (
        glib::g_log!(module_path!(), glib::LogLevel::Debug, $($arg)+)
    )
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)+) => (
        glib::g_log!(module_path!(), glib::LogLevel::Debug, $($arg)+)
    )
}
