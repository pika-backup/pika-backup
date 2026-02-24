//! User interface

const NON_JOURNALING_FILESYSTEMS: &[&str] = &["exfat", "ext2", "vfat"];

mod action;
mod app;
mod backup_status;
mod dbus;
mod error;
mod globals;
mod operation;
mod prelude;
mod repo;
mod shell;
mod status;
mod toast_size_estimate;
mod utils;
mod widget;

pub use app::App;
use common::{borg, config};
pub(crate) use globals::{BACKUP_CONFIG, BACKUP_HISTORY};
use gtk::prelude::*;
use gvdb_macros::include_gresource_from_dir;

use crate::prelude::*;

static GRESOURCE_BYTES: &[u8] =
    if const_str::equal!("/org/gnome/World/PikaBackup", common::DBUS_API_PATH) {
        include_gresource_from_dir!("/org/gnome/World/PikaBackup", "data/resources")
    } else if const_str::equal!("/org/gnome/World/PikaBackup/Devel", common::DBUS_API_PATH) {
        include_gresource_from_dir!("/org/gnome/World/PikaBackup/Devel", "data/resources")
    } else {
        panic!("Invalid DBUS_API_PATH")
    };

// Run application
pub fn main() {
    common::utils::init_logging("pika-backup");
    common::utils::init_gettext();

    // Ctrl-C handling
    glib::unix_signal_add(nix::sys::signal::Signal::SIGINT as i32, on_ctrlc);

    gio::resources_register(
        &gio::Resource::from_data(&glib::Bytes::from_static(GRESOURCE_BYTES)).unwrap(),
    );

    let app = App::new();
    app.run();
}

fn on_ctrlc() -> glib::ControlFlow {
    tracing::debug!("Quit: SIGINT (Ctrl+C)");

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

            if let Some(version) = version_output
                .lines()
                .next()
                .and_then(|x| x.split(' ').nth(1))
                .and_then(|v| {
                    // Parse three numbers to a vec
                    v.splitn(3, '.')
                        .map(str::parse::<u32>)
                        .collect::<std::result::Result<Vec<_>, _>>()
                        .ok()
                })
            {
                let version_format = |version: &[u32]| {
                    version
                        .iter()
                        .map(ToString::to_string)
                        .reduce(|acc, s| format!("{acc}.{s}"))
                        .unwrap_or_default()
                };

                if version[..] < borg::MIN_VERSION[..] {
                    return Err(Message::new(
                            gettext("BorgBackup Version Too Old"),
                            gettextf(
                                "The installed version {} of BorgBackup is older than the required version {}. This is unsupported.",
                                [
                                    &version_format(&version),
                                    &version_format(&borg::MIN_VERSION),
                                ],
                            )).into());
                } else if version[..2] > borg::MAX_VERSION[..] {
                    // Ignore patch version for maximum version determination
                    return Err(Message::new(
                            gettext("BorgBackup Version Too New"),
                            gettextf(
                                "The installed version {} of BorgBackup is new and untested. Version {} is recommended.",
                                [
                                    &version_format(&version),
                                    &version_format(&borg::MAX_VERSION),
                                ],
                            )).into());
                }
            } else {
                return Err(Message::new(
                    gettext("Failed to Check BorgBackup Version"),
                    gettextf(
                        "The installed version {} might not work.",
                        [&version_output],
                    ),
                )
                .into());
            }
        }
    }

    Ok(())
}
