pub use crate::ui::utils::BackupMap;
pub use gettextrs::gettext;

use arc_swap::ArcSwap;

use chrono::prelude::*;
use gtk::prelude::*;

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
    fn update<F: Fn(&mut T)>(&self, updater: F);
    fn get(&self) -> T;
}

impl<T> ArcSwapAdditions<T> for ArcSwap<T>
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

impl<T> ArcSwapAdditions<T> for once_cell::sync::Lazy<ArcSwap<T>>
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

pub trait CronoAdditions {
    fn to_locale(&self) -> Option<String>;
}

impl CronoAdditions for NaiveDateTime {
    fn to_locale(&self) -> Option<String> {
        let dt = chrono::Local.from_local_datetime(&self).earliest()?;
        let gdt = glib::DateTime::from_unix_local(dt.timestamp());
        Some(gdt.format("%c")?.to_string())
    }
}

pub fn gettextf(format: &str, args: &[&str]) -> String {
    let mut s = gettext(format);

    for arg in args {
        s = s.replacen("{}", arg, 1)
    }
    s
}

pub trait WidgetEnh {
    fn add_css_class(&self, class: &str);
    fn remove_css_class(&self, class: &str);
}

impl<W: gtk::WidgetExt> WidgetEnh for W {
    fn add_css_class(&self, class: &str) {
        self.get_style_context().add_class(class);
    }

    fn remove_css_class(&self, class: &str) {
        self.get_style_context().remove_class(class);
    }
}
