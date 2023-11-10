pub mod dbus;
pub mod host;
pub mod password;
pub mod upower;

use crate::config;
use crate::prelude::*;
use async_std::prelude::*;

use gio::prelude::*;
use std::convert::TryInto;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

pub trait LookupConfigId {
    type Item;

    fn try_get_mut(
        &mut self,
        key: &ConfigId,
    ) -> Result<&mut Self::Item, config::error::BackupNotFound>;
    fn try_get(&self, key: &ConfigId) -> Result<&Self::Item, config::error::BackupNotFound>;
}

extern "C" {
    fn fnmatch(pattern: *const c_char, string: *const c_char, flags: c_int) -> c_int;
}

pub fn posix_fnmatch(pattern: &CStr, string: &CStr) -> bool {
    unsafe { fnmatch(pattern.as_ptr(), string.as_ptr(), 0) == 0 }
}

pub fn init_gettext() {
    gettextrs::setlocale(gettextrs::LocaleCategory::LcAll, "");
    debug!(
        "bindtextdomain: {:?}",
        gettextrs::bindtextdomain(env!("CARGO_PKG_NAME"), crate::LOCALEDIR)
    );
    debug!(
        "textdomain: {:?}",
        gettextrs::textdomain(env!("CARGO_PKG_NAME"))
    );
}

pub async fn listen_remote_app_running<T: Fn(bool)>(
    app_id: &str,
    update: T,
) -> Result<(), zbus::Error> {
    let proxy = crate::utils::dbus::fdo_proxy().await?;

    let has_owner = proxy.name_has_owner(app_id.try_into().unwrap()).await?;
    update(has_owner);

    let mut stream = proxy.receive_name_owner_changed().await?;

    while let Some(signal) = stream.next().await {
        let args = signal.args()?;
        if args.name == app_id {
            debug!(
                "Remote app '{}' owner status changed: {:?}",
                args.name, args.new_owner
            );
            update(args.new_owner.is_some());
        }
    }

    Ok(())
}

pub fn file_symbolic_icon(path: &std::path::Path) -> Option<gtk::Image> {
    let file = gio::File::for_path(path);
    let info = file.query_info("*", gio::FileQueryInfoFlags::NONE, gio::Cancellable::NONE);
    match info {
        Ok(info) => info.symbolic_icon().as_ref().map(gtk::Image::from_gicon),
        Err(err) if matches!(err.kind(), Some(gio::IOErrorEnum::NotFound)) => Some(
            gtk::Image::builder()
                .icon_name("warning-symbolic")
                .tooltip_text(gettext("No such file or directory"))
                .build(),
        ),
        Err(_) => None,
    }
}
