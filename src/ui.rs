//! User interface

mod actions;
mod backup_status;
#[allow(dead_code)]
mod builder;
mod dbus;
mod dialog_about;
mod dialog_archive_prefix;
mod dialog_check;
mod dialog_delete_archive;
mod dialog_device_missing;
mod dialog_encryption_password;
mod dialog_exclude;
mod dialog_exclude_pattern;
mod dialog_preferences;
mod dialog_prune;
mod dialog_prune_review;
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

pub(crate) use globals::{BACKUP_CONFIG, BACKUP_HISTORY, SCHEDULE_STATUS};

use gtk::prelude::*;
use gvdb_macros::include_gresource_from_dir;

use crate::borg;
use crate::config;
use crate::ui;
use crate::ui::prelude::*;
use config::TrackChanges;

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

fn on_ctrlc() -> glib::ControlFlow {
    debug!("Quit: SIGINT (Ctrl+C)");

    BORG_OPERATION.with(|operations| {
        for op in operations.load().values() {
            op.set_instruction(borg::Instruction::Abort(borg::Abort::User));
        }
    });

    adw_app().quit();
    glib::ControlFlow::Continue
}

fn on_shutdown(_app: &adw::Application) {
    IS_SHUTDOWN.swap(std::sync::Arc::new(true));

    let result = BACKUP_HISTORY.try_update(|histories| {
        config::Histories::handle_shutdown(histories);
        Ok(())
    });

    if let Err(err) = result {
        error!("Failed to write config during shutdown: {}", err);
    }

    while !ACTIVE_MOUNTS.load().is_empty() {
        async_std::task::block_on(async {
            for repo_id in ACTIVE_MOUNTS.load().iter() {
                if borg::functions::umount(repo_id).await.is_ok() {
                    ACTIVE_MOUNTS.update(|mounts| {
                        mounts.remove(repo_id);
                    });
                }
            }
        })
    }

    debug!("Good bye!");
}

fn on_startup(_app: &adw::Application) {
    debug!("Signal 'startup'");
    ui::utils::config_io::load_config();
    config::ScheduleStatus::update_on_change(&SCHEDULE_STATUS, |err| {
        Err::<(), std::io::Error>(err).handle("Failed to load Schedule Status")
    })
    .handle("Failed to Load Schedule Status");

    // Force adwaita icon theme
    if let Some(settings) = gtk::Settings::default() {
        settings.set_property("gtk-icon-theme-name", "Adwaita");
    }

    ui::actions::init();
    glib::MainContext::default().spawn_local(async {
        ui::dbus::init().await;
    });

    main_ui();

    // init status tracking
    status_tracking();

    adw_app().set_accels_for_action("app.help", &["F1"]);
    adw_app().set_accels_for_action("app.quit", &["<Ctrl>Q"]);
    adw_app().set_accels_for_action("app.setup", &["<Ctrl>N"]);
    adw_app().set_accels_for_action("app.backup-preferences", &["<Ctrl>comma"]);
    adw_app().set_accels_for_action("win.show-help-overlay", &["<Ctrl>question"]);

    if BACKUP_CONFIG.load().iter().count() == 1 {
        if let Some(config) = BACKUP_CONFIG.load().iter().next() {
            main_ui()
                .page_detail()
                .backup_page()
                .view_backup_conf(&config.id);
        }
    }
}

fn on_activate(_app: &adw::Application) {
    debug!("Signal 'activate'");
    main_ui().present();
}

async fn quit() -> Result<()> {
    debug!("Running quit routine");
    if utils::borg::is_borg_operation_running() {
        if main_ui().window().is_visible() {
            let permission = utils::background_permission().await;

            match permission {
                Ok(()) => {
                    debug!("Hiding main window as backup is currently running");
                    main_ui().window().set_visible(false);
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
                    quit_real().await;
                }
            }
        } else {
            // Someone wants to quit the app from the shell (eg via backgrounds app list)
            // Or we do something wrong and called this erroneously
            debug!("Received quit request while a backup operation is running. Ignoring");
            let notification = gio::Notification::new(&gettext("A Backup Operation is Running"));
            notification.set_body(Some(&gettext(
                "Pika Backup cannot be quit during a backup operation.",
            )));

            adw_app().send_notification(None, &notification);
        }
    } else {
        quit_real().await;
    }

    Ok(())
}

async fn quit_real() {
    shell::set_status_message(&gettext("Quit")).await;

    adw_app().quit();
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
