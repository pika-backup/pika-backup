pub use crate::ui::utils::BackupMap;
pub use gettextrs::*;

use arc_swap::ArcSwap;

use chrono::prelude::*;

pub trait Humanize {
    fn humanize(self) -> String;
}

impl Humanize for std::time::Duration {
    fn humanize(self) -> String {
        if let Ok(duration) = chrono::Duration::from_std(self) {
            duration.humanize()
        } else {
            String::from("Too large")
        }
    }
}

impl Humanize for chrono::Duration {
    fn humanize(self) -> String {
        chrono_humanize::HumanTime::from(self).to_string()
    }
}

pub trait ArcSwapAdditions<T> {
    fn update<F: Fn(&mut T) -> ()>(&self, updater: F);
    fn get(&self) -> T;
}

impl<T> ArcSwapAdditions<T> for ArcSwap<T>
where
    T: Clone,
{
    fn update<F: Fn(&mut T) -> ()>(&self, updater: F) {
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

impl<T> ArcSwapAdditions<T> for once_cell::sync::Lazy<ArcSwap<T>>
where
    T: Clone,
{
    fn update<F: Fn(&mut T) -> ()>(&self, updater: F) {
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

pub trait CronoAdditions {
    fn to_glib(&self) -> glib::DateTime;
    fn to_locale(&self) -> String;
}

impl CronoAdditions for NaiveDateTime {
    fn to_glib(&self) -> glib::DateTime {
        glib::DateTime::from_unix_local(self.timestamp())
    }

    fn to_locale(&self) -> String {
        self.to_glib()
            .format("%c")
            .map(|gstr| gstr.to_string())
            .unwrap_or_else(|| self.format("%c").to_string())
    }
}

impl CronoAdditions for DateTime<Local> {
    fn to_glib(&self) -> glib::DateTime {
        glib::DateTime::from_unix_local(self.timestamp())
    }

    fn to_locale(&self) -> String {
        self.to_glib()
            .format("%c")
            .map(|gstr| gstr.to_string())
            .unwrap_or_else(|| self.format("%c").to_string())
    }
}
