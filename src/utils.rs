pub mod host;
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

    fn get_result_mut(
        &mut self,
        key: &ConfigId,
    ) -> Result<&mut Self::Item, config::error::BackupNotFound>;
    fn get_result(&self, key: &ConfigId) -> Result<&Self::Item, config::error::BackupNotFound>;
}

extern "C" {
    fn fnmatch(pattern: *const c_char, string: *const c_char, flags: c_int) -> c_int;
}

pub fn posix_fnmatch(pattern: &CStr, string: &CStr) -> bool {
    unsafe { fnmatch(pattern.as_ptr(), string.as_ptr(), 0) == 0 }
}

pub fn mount_uuid(mount: &gio::Mount) -> Option<String> {
    let volume = mount.volume();

    volume
        .as_ref()
        .and_then(gio::Volume::uuid)
        .or_else(|| volume.as_ref().and_then(|v| v.identifier("uuid")))
        .as_ref()
        .map(std::string::ToString::to_string)
}

pub fn init_gettext() {
    gettextrs::setlocale(gettextrs::LocaleCategory::LcAll, "");
    let localedir = option_env!("LOCALEDIR").unwrap_or(crate::DEFAULT_LOCALEDIR);
    debug!(
        "bindtextdomain: {:?}",
        gettextrs::bindtextdomain(env!("CARGO_PKG_NAME"), localedir)
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
    let proxy = zbus::fdo::DBusProxy::new(&ZBUS_SESSION).await?;

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

pub fn file_symbolic_icon(path: &std::path::Path) -> Option<gio::Icon> {
    let file = gio::File::for_path(path);
    let info = file.query_info("*", gio::FileQueryInfoFlags::NONE, gio::Cancellable::NONE);
    if let Ok(info) = info {
        info.symbolic_icon()
    } else {
        None
    }
}
