//! User interface

mod about;
mod app;
mod backup_status;
#[allow(dead_code)]
mod builder;
mod dbus;
mod dialog_delete_archive;
mod dialog_device_missing;
mod dialog_encryption_password;
mod dialog_exclude_pattern;
mod dialog_preferences;
mod dialog_setup;
mod dialog_storage;
mod error;
mod export;
mod globals;
mod operation;
mod prelude;
mod shell;
mod status;
mod toast_size_estimate;
mod utils;
mod widget;

pub use app::App;
pub(crate) use globals::{BACKUP_CONFIG, BACKUP_HISTORY, SCHEDULE_STATUS};

use gtk::prelude::*;
use gvdb_macros::include_gresource_from_dir;

use crate::borg;
use crate::config;
use crate::ui;
use crate::ui::prelude::*;

static GRESOURCE_BYTES: &[u8] =
    if const_str::equal!("/org/gnome/World/PikaBackup", crate::DBUS_API_PATH) {
        include_gresource_from_dir!("/org/gnome/World/PikaBackup", "data/resources")
    } else if const_str::equal!("/org/gnome/World/PikaBackup/Devel", crate::DBUS_API_PATH) {
        include_gresource_from_dir!("/org/gnome/World/PikaBackup/Devel", "data/resources")
    } else {
        panic!("Invalid DBUS_API_PATH")
    };

// Run application
pub fn main() {
    if std::env::var_os("ZBUS_TRACING").map_or(false, |x| !x.is_empty()) {
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing_subscriber::filter::LevelFilter::TRACE)
            .init();
    }

    crate::utils::init_gettext();

    // Ctrl-C handling
    glib::unix_signal_add(nix::sys::signal::Signal::SIGINT as i32, on_ctrlc);

    gio::resources_register(
        &gio::Resource::from_data(&glib::Bytes::from_static(GRESOURCE_BYTES)).unwrap(),
    );

    let app = App::new();
    app.run();
}

fn on_ctrlc() -> glib::ControlFlow {
    debug!("Quit: SIGINT (Ctrl+C)");

    BORG_OPERATION.with(|operations| {
        for op in operations.load().values() {
            op.set_instruction(borg::Instruction::Abort(borg::Abort::User));
        }
    });

    Handler::run(async move { adw_app().try_quit().await });
    glib::ControlFlow::Continue
}

async fn init_check_borg() -> Result<()> {
    let version_result = utils::spawn_thread("borg::version", borg::version)
        .await?
        .await;

    match version_result {
        Err(err) => {
            let _ = globals::BORG_VERSION.set(format!("Error: {}", err));

            return Err(Message::new(
                gettext("Failed to run “borg”. Is BorgBackup installed correctly?"),
                err,
            )
            .into());
        }
        Ok(version_output) => {
            let _ = globals::BORG_VERSION.set(version_output.clone());

            if let Some(version_string) = version_output
                .lines()
                .next()
                .and_then(|x| x.split(' ').nth(1))
            {
                let mut version_list = version_string.split('.').map(str::parse::<u32>);

                if let (Some(Ok(major)), Some(Ok(minor)), Some(Ok(patch))) = (
                    version_list.next(),
                    version_list.next(),
                    version_list.next(),
                ) {
                    #[allow(clippy::absurd_extreme_comparisons)]
                    if major < borg::MIN_MAJOR_VERSION
                        || minor < borg::MIN_MINOR_VERSION
                        || patch < borg::MIN_PATCH_VERSION
                    {
                        return Err(Message::new(
                    gettext("BorgBackup version too old"),
                    gettextf(
                        "The installed version {} of BorgBackup is too old. Some features requiring borg-backup version {}.{}.{} will not work.",
                        &[
                            &version_output,
                            &borg::MIN_MAJOR_VERSION.to_string(),
                            &borg::MIN_MINOR_VERSION.to_string(),
                            &borg::MIN_PATCH_VERSION.to_string(),
                        ],
                    )).into());
                    }
                    if major > borg::MAX_MAJOR_VERSION || minor > borg::MAX_MINOR_VERSION {
                        return Err(Message::new(
                    gettext("BorgBackup version too new"),
                    gettextf(
                        "The installed version {} of BorgBackup is too new. Version {}.{} is recommended. Some features might not work as expected.",
                        &[
                            &version_output,
                            &borg::MAX_MAJOR_VERSION.to_string(),
                            &borg::MAX_MINOR_VERSION.to_string(),
                        ],
                    )).into());
                    }
                } else {
                    return Err(Message::new(
                        gettext("Failed to Check BorgBackup Version"),
                        gettextf(
                            "The installed version {} might not work.",
                            &[&version_output],
                        ),
                    )
                    .into());
                }
            }
        }
    }

    Ok(())
}
