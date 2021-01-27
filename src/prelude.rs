pub use crate::globals::*;
pub use gtk::gdk;
pub use gtk::gdk_pixbuf;
pub use gtk::gio;
pub use gtk::glib;
pub use gtk::glib::signal::Inhibit;
pub use gtk::pango;

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
        gtk::glib::g_log!(module_path!(), gtk::glib::LogLevel::Error, $($arg)+)
    )
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)+) => (
        gtk::glib::g_log!(module_path!(), gtk::glib::LogLevel::Warning, $($arg)+)
    )
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)+) => (
        gtk::glib::g_log!(module_path!(), gtk::glib::LogLevel::Info, $($arg)+)
    )
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)+) => (
        gtk::glib::g_log!(module_path!(), gtk::glib::LogLevel::Debug, $($arg)+)
    )
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)+) => (
        gtk::glib::g_log!(module_path!(), gtk::glib::LogLevel::Debug, $($arg)+)
    )
}
