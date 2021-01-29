pub use crate::globals::*;

use arc_swap::ArcSwap;

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

impl<T> ArcSwapExt<T> for once_cell::sync::Lazy<ArcSwap<T>>
where
    T: Clone,
{
    fn update<F: Fn(&mut T)>(&self, updater: F) {
        (**self).rcu(|current| {
            let mut new = T::clone(current);
            updater(&mut new);
            new
        });
    }

    fn get(&self) -> T {
        T::clone(&(**self).load_full())
    }
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)+) => (
        glib::g_log!(module_path!(), glib::LogLevel::Error, $($arg)+)
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
