//! User interface

mod actions;
mod app_window;
mod backup_status;
#[allow(dead_code)]
mod builder;
mod dbus;
mod dialog_about;
mod dialog_archive_prefix;
mod dialog_delete_archive;
mod dialog_device_missing;
mod dialog_encryption_password;
mod dialog_exclude;
mod dialog_exclude_pattern;
mod dialog_info;
mod dialog_prune;
mod dialog_prune_review;
mod dialog_setup;
mod dialog_storage;
mod error;
mod export;
mod globals;
mod headerbar;
mod operation;
mod page_archives;
mod page_backup;
mod page_detail;
mod page_overview;
mod page_schedule;
mod prelude;
mod shell;
mod status;
mod toast_size_estimate;
mod utils;
mod widget;

pub(crate) use globals::{BACKUP_CONFIG, BACKUP_HISTORY, SCHEDULE_STATUS};

use gtk::prelude::*;
use gvdb_macros::include_gresource_from_dir;

use crate::borg;
use crate::config;
use crate::ui;
use crate::ui::prelude::*;
use config::TrackChanges;
use utils::config_io::write_config;

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
    crate::utils::init_gettext();

    adw_app().connect_startup(on_startup);
    adw_app().connect_activate(on_activate);
    adw_app().connect_shutdown(on_shutdown);

    // Ctrl-C handling
    glib::unix_signal_add(nix::sys::signal::Signal::SIGINT as i32, on_ctrlc);

    gio::resources_register(
        &gio::Resource::from_data(&glib::Bytes::from_static(GRESOURCE_BYTES)).unwrap(),
    );

    adw_app().run();
}

fn on_ctrlc() -> Continue {
    debug!("Quit: SIGINT (Ctrl+C)");

    BORG_OPERATION.with(|operations| {
        for op in operations.load().values() {
            op.set_instruction(borg::Instruction::Abort(borg::Abort::User));
        }
    });

    adw_app().quit();
    Continue(true)
}

fn on_shutdown(_app: &adw::Application) {
    IS_SHUTDOWN.swap(std::sync::Arc::new(true));

    BACKUP_HISTORY.update(|histories| {
        config::Histories::handle_shutdown(histories);
    });
    if let Err(err) = write_config() {
        error!("Failed to write config during shutdown: {}", err);
    }

    while !ACTIVE_MOUNTS.load().is_empty() {
        for repo_id in ACTIVE_MOUNTS.load().iter() {
            if borg::functions::umount(repo_id).is_ok() {
                ACTIVE_MOUNTS.update(|mounts| {
                    mounts.remove(repo_id);
                });
            }
        }
    }

    debug!("Good bye!");
}

fn on_startup(_app: &adw::Application) {
    debug!("Signal 'startup'");
    ui::utils::config_io::load_config();
    config::ScheduleStatus::update_on_change(&SCHEDULE_STATUS)
        .handle("Failed to Load Schedule Status");

    // Force adwaita icon theme
    if let Some(settings) = gtk::Settings::default() {
        settings.set_property("gtk-icon-theme-name", "Adwaita");
    }

    ui::actions::init();
    ui::dbus::init();

    ui::app_window::init();
    ui::headerbar::init();

    ui::page_overview::init();

    ui::page_detail::init();
    ui::page_backup::init::init();
    ui::page_archives::init();
    ui::page_schedule::init::init();

    // init status tracking
    status_tracking();

    adw_app().set_accels_for_action("app.help", &["F1"]);
    adw_app().set_accels_for_action("app.quit", &["<Ctrl>Q"]);
    adw_app().set_accels_for_action("app.setup", &["<Ctrl>N"]);
    adw_app().set_accels_for_action("win.show-help-overlay", &["<Ctrl>question"]);

    if BACKUP_CONFIG.load().iter().count() > 1 {
        main_ui()
            .main_stack()
            .set_visible_child(&main_ui().page_overview());
    } else if let Some(config) = BACKUP_CONFIG.load().iter().next() {
        ui::page_backup::view_backup_conf(&config.id);
    } else {
        main_ui()
            .main_stack()
            .set_visible_child(&main_ui().page_overview_empty());
    }

    Handler::handle(ui::utils::fix_flatpak_autostart());
}

fn on_activate(_app: &adw::Application) {
    debug!("Signal 'activate'");
    app_window::show();
}

async fn quit() -> Result<()> {
    debug!("Running quit routine");
    if utils::borg::is_backup_running() {
        let permission = utils::background_permission().await;

        match permission {
            Ok(()) => {
                main_ui().window().hide();
            }
            Err(err) => {
                err.show().await;

                ui::utils::confirmation_dialog(
                    &gettext("Abort running backup creation?"),
                    &gettext("The backup will remain incomplete if aborted now."),
                    &gettext("Continue"),
                    &gettext("Abort"),
                )
                .await?;
                adw_app().quit();
            }
        }
    } else {
        adw_app().quit();
    }

    Ok(())
}

async fn init_check_borg() -> Result<()> {
    let version_result = utils::spawn_thread("borg::version", borg::version).await?;

    match version_result {
        Err(err) => {
            return Err(Message::new(
                gettext("Failed to run “borg”. Is BorgBackup installed correctly?"),
                err,
            )
            .into());
        }
        Ok(version_output) => {
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
