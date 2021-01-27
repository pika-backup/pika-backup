pub use crate::prelude::*;
pub use crate::ui::error::{Error, Handler, Result, UserAborted};
pub use crate::ui::globals::*;
pub use crate::ui::utils::{BackupMap, Message};

pub use gettextrs::gettext;

use chrono::prelude::*;
use gtk::prelude::*;

pub fn spawn_local<F: std::future::Future<Output = ()> + 'static>(f: F) {
    glib::MainContext::default().spawn_local(f);
}

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

pub trait CronoExt {
    fn to_glib(&self) -> glib::DateTime;
    fn to_locale(&self) -> String;
}

// TODO gtk4
impl CronoExt for NaiveDateTime {
    fn to_glib(&self) -> glib::DateTime {
        glib::DateTime::from_unix_local(self.timestamp()).unwrap()
    }

    fn to_locale(&self) -> String {
        self.to_glib()
            .format("%c")
            .map(|gstr| gstr.to_string())
            .unwrap()
        //.unwrap_or_else(|| self.format("%c").to_string())
    }
}

impl CronoExt for DateTime<Local> {
    fn to_glib(&self) -> glib::DateTime {
        glib::DateTime::from_unix_local(self.timestamp()).unwrap()
    }

    fn to_locale(&self) -> String {
        self.to_glib()
            .format("%c")
            .map(|gstr| gstr.to_string())
            .unwrap()
        //.unwrap_or_else(|| self.format("%c").to_string())
    }
}

pub fn gettextf(format: &str, args: &[&str]) -> String {
    let mut s = gettext(format);

    for arg in args {
        s = s.replacen("{}", arg, 1)
    }
    s
}
