pub use crate::prelude::*;
pub use crate::ui::error::{Error, ErrorToMessage, Handler, Message, Result, UserAborted};
pub use crate::ui::globals::*;
pub use crate::ui::utils::ext::*;
pub use crate::ui::utils::BackupMap;

pub use gettextrs::gettext;

pub fn spawn_local<F: std::future::Future<Output = ()> + 'static>(f: F) {
    glib::MainContext::default().spawn_local(f);
}

pub fn gettextf(format: &str, args: &[&str]) -> String {
    let mut s = gettext(format);

    for arg in args {
        s = s.replacen("{}", arg, 1)
    }
    s
}
