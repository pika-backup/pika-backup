pub use std::convert::TryFrom;
pub use std::rc::Rc;
pub use std::sync::Arc;
pub use std::time::Duration;

use arc_swap::ArcSwap;
pub use gettextrs::{gettext, ngettext};

use crate::config;
pub use crate::config::ConfigId;
pub use crate::globals::*;
pub use crate::utils::LookupConfigId;

pub fn gettextf(format: &str, args: impl IntoIterator<Item = impl std::fmt::Display>) -> String {
    let mut s = gettext(format);

    for arg in args {
        s = s.replacen("{}", &arg.to_string(), 1)
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
    fn try_get_mut(&mut self, key: &ConfigId) -> Result<&mut T, config::error::BackupNotFound> {
        self.get_mut(key)
            .ok_or_else(|| config::error::BackupNotFound::new(key.clone()))
    }

    fn try_get(&self, key: &ConfigId) -> Result<&T, config::error::BackupNotFound> {
        self.get(key)
            .ok_or_else(|| config::error::BackupNotFound::new(key.clone()))
    }
}

pub trait ArcSwapUpdate<T> {
    /// Clone and update the inner value with the provided closure
    fn update<F: Fn(&mut T)>(&self, updater: F);
}

impl<T> ArcSwapUpdate<T> for ArcSwap<T>
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
}

pub trait ArcSwapUpdateWriteable<T> {
    fn update_no_commit<F: Fn(&mut T)>(&self, updater: F);
}

impl<T> ArcSwapUpdateWriteable<T> for ArcSwap<config::Writeable<T>>
where
    T: Clone,
{
    /// Update the inner value with the provided closure. Doesn't save the
    /// writeable.
    fn update_no_commit<F: Fn(&mut T)>(&self, updater: F) {
        self.rcu(|current| {
            let mut new = T::clone(&current.current_config);
            updater(&mut new);

            crate::config::Writeable {
                current_config: new,
                written_config: current.written_config.clone(),
            }
        });
    }
}

pub trait ArcSwapGet<T> {
    fn get(&self) -> T;
}

impl<T> ArcSwapGet<T> for ArcSwap<T>
where
    T: Clone,
{
    fn get(&self) -> T {
        T::clone(&self.load_full())
    }
}

impl<T> ArcSwapGet<T> for ArcSwap<config::Writeable<T>>
where
    T: Clone,
{
    fn get(&self) -> T {
        T::clone(&self.load().current_config)
    }
}
