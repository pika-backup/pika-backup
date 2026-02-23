pub mod action;
pub mod dbus;
pub mod host;
pub mod password;
pub mod upower;

use std::convert::TryInto;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

use gio::prelude::*;
use smol::prelude::*;
use tracing_subscriber::prelude::*;

use crate::config;
use crate::prelude::*;

pub trait LookupConfigId {
    type Item;

    fn try_get_mut(
        &mut self,
        key: &ConfigId,
    ) -> Result<&mut Self::Item, config::error::BackupNotFound>;
    fn try_get(&self, key: &ConfigId) -> Result<&Self::Item, config::error::BackupNotFound>;
}

unsafe extern "C" {
    fn fnmatch(pattern: *const c_char, string: *const c_char, flags: c_int) -> c_int;
}

pub fn posix_fnmatch(pattern: &CStr, string: &CStr) -> bool {
    unsafe { fnmatch(pattern.as_ptr(), string.as_ptr(), 0) == 0 }
}

pub fn init_gettext() {
    gettextrs::setlocale(gettextrs::LocaleCategory::LcAll, "");
    tracing::debug!(
        "bindtextdomain: {:?}",
        gettextrs::bindtextdomain("pika-backup", crate::LOCALEDIR)
    );
    tracing::debug!("textdomain: {:?}", gettextrs::textdomain("pika-backup"));
}

pub fn init_logging(domain: &str) {
    // Follow G_MESSAGES_DEBUG env variable
    let default_level = if !glib::log_writer_default_would_drop(glib::LogLevel::Debug, Some(domain))
    {
        tracing_subscriber::filter::LevelFilter::DEBUG
    } else {
        tracing_subscriber::filter::LevelFilter::ERROR
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(default_level.into())
                .from_env_lossy(),
        )
        .with(tracing_subscriber::fmt::Layer::default().compact())
        .init();

    tracing::debug!("Logger initialized");
}

pub async fn listen_remote_app_running<T: Fn(bool)>(
    app_id: &str,
    session_connection: &zbus::Connection,
    update: T,
) -> Result<(), zbus::Error> {
    let proxy = crate::utils::dbus::fdo_proxy(session_connection).await?;

    let has_owner = proxy.name_has_owner(app_id.try_into().unwrap()).await?;
    update(has_owner);

    let mut stream = proxy.receive_name_owner_changed().await?;

    while let Some(signal) = stream.next().await {
        let args = signal.args()?;
        if args.name == app_id {
            tracing::debug!(
                "Remote app '{}' owner status changed: {:?}",
                args.name,
                args.new_owner
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
