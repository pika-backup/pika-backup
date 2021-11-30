pub use crate::config::ConfigId;
pub use crate::globals::*;
pub use crate::utils::LookupConfigId;
pub use std::convert::TryFrom;
pub use std::rc::Rc;
pub use std::sync::Arc;
pub use std::time::Duration;

use crate::config;

use arc_swap::ArcSwap;

pub use gettextrs::{gettext, ngettext};

pub fn gettextf(format: &str, args: &[&str]) -> String {
    let mut s = gettext(format);

    for arg in args {
        s = s.replacen("{}", arg, 1)
    }
    s
}

pub fn ngettextf(msgid: &str, msgid_plural: &str, n: u32, args: &[&str]) -> String {
    let mut s = ngettext(msgid, msgid_plural, n);

    for arg in args {
        s = s.replacen("{}", arg, 1)
    }
    s
}

pub fn ngettextf_(msgid: &str, msgid_plural: &str, n: u32) -> String {
    ngettextf(msgid, msgid_plural, n, &[&n.to_string()])
}

#[allow(clippy::implicit_hasher)]
impl<T> LookupConfigId for std::collections::BTreeMap<ConfigId, T> {
    type Item = T;
    fn get_result_mut(&mut self, key: &ConfigId) -> Result<&mut T, config::error::BackupNotFound> {
        self.get_mut(key)
            .ok_or_else(|| config::error::BackupNotFound::new(key.clone()))
    }

    fn get_result(&self, key: &ConfigId) -> Result<&T, config::error::BackupNotFound> {
        self.get(key)
            .ok_or_else(|| config::error::BackupNotFound::new(key.clone()))
    }
}

pub trait ArcSwapExt<T> {
    fn update<F: Fn(&mut T)>(&self, updater: F);
    fn update_return<R, F: Fn(&mut T) -> R>(&self, updater: F) -> R;
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

    fn update_return<R, F: Fn(&mut T) -> R>(&self, updater: F) -> R {
        let mut cell = once_cell::sync::OnceCell::new();

        self.rcu(|current| {
            let mut new = T::clone(current);
            let _set = cell.set(updater(&mut new));
            new
        });

        cell.take().unwrap()
    }

    fn get(&self) -> T {
        T::clone(&self.load_full())
    }
}

impl<T> ArcSwapExt<T> for ArcSwap<config::Writeable<T>>
where
    T: Clone,
{
    fn update<F: Fn(&mut T)>(&self, updater: F) {
        self.rcu(|current| {
            let mut new = T::clone(&current.current_config);
            updater(&mut new);

            config::Writeable {
                current_config: new,
                written_config: current.written_config.clone(),
            }
        });
    }

    fn update_return<R, F: Fn(&mut T) -> R>(&self, updater: F) -> R {
        let mut cell = once_cell::sync::OnceCell::new();

        self.rcu(|current| {
            let mut new = T::clone(&current.current_config);
            let _set = cell.set(updater(&mut new));

            config::Writeable {
                current_config: new,
                written_config: current.written_config.clone(),
            }
        });

        cell.take().unwrap()
    }

    fn get(&self) -> T {
        T::clone(&self.load().current_config)
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
