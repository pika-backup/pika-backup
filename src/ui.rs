use gdk_pixbuf::prelude::*;
use gio::prelude::*;
use gtk::prelude::*;

use crate::borg;
use crate::config;
use crate::ui;
use crate::ui::prelude::*;

mod app_window;
mod backup_status;
#[allow(dead_code)]
#[allow(clippy::new_without_default)]
mod builder;
mod dialog_about;
mod dialog_add_config;
mod dialog_device_missing;
mod dialog_encryption_password;
mod dialog_info;
mod dialog_storage;
mod error;
mod globals;
mod headerbar;
mod page_archives;
mod page_detail;
mod page_overview;
mod page_pending;
mod prelude;
mod update_config;
mod utils;

pub fn main() {
    // suppress "gdk_pixbuf_from_pixdata()" debug spam
    glib::log_set_handler(
        Some("GdkPixbuf"),
        glib::LogLevels::LEVEL_DEBUG,
        false,
        false,
        |_, _, _| {},
    );

    // init gettext
    if gettextrs::setlocale(gettextrs::LocaleCategory::LcAll, "").is_none() {
        warn!("gettextrs::setlocale() failed");
    }
    let localedir = option_env!("LOCALEDIR").unwrap_or(crate::DEFAULT_LOCALEDIR);
    info!("bindtextdomain setting directory to {:?}", localedir);
    if let Err(err) = gettextrs::bindtextdomain(env!("CARGO_PKG_NAME"), localedir) {
        warn!("gettextrs::bindtextdomain() failed: {}", err);
    }
    if let Err(err) = gettextrs::textdomain(env!("CARGO_PKG_NAME")) {
        warn!("gettextrs::textdomain() failed: {}", err);
    }

    // init gtk and libhandy
    gtk::init().expect("Failed to gtk::init()");
    libhandy::init();

    gtk_app().connect_startup(on_startup);
    gtk_app().connect_activate(on_activate);
    gtk_app().connect_shutdown(on_shutdown);

    crate::globals::init();

    // Ctrl-C handling
    glib::unix_signal_add(nix::sys::signal::Signal::SIGINT as i32, on_ctrlc);

    gtk_app().run(&std::env::args().collect::<Vec<_>>());
}

fn on_ctrlc() -> Continue {
    debug!("Quit: SIGINT (Ctrl+C)");

    for com in BACKUP_COMMUNICATION.load().values() {
        com.instruction.update(|instruction| {
            *instruction = borg::Instruction::Abort;
        })
    }

    gtk_app().release();
    Continue(true)
}

fn on_shutdown(app: &gtk::Application) {
    app.mark_busy();
    IS_SHUTDOWN.swap(std::sync::Arc::new(true));
    while !ACTIVE_MOUNTS.load().is_empty() {
        for repo_id in ACTIVE_MOUNTS.load().iter() {
            if borg::Borg::umount(&repo_id).is_ok() {
                ACTIVE_MOUNTS.update(|mounts| {
                    mounts.remove(&repo_id);
                });
            }
        }
    }

    debug!("Good bye!");
}

fn on_startup(_app: &gtk::Application) {
    debug!("Signal 'startup'");
    load_config();

    if let Some(screen) = gdk::Screen::get_default() {
        let provider = gtk::CssProvider::new();
        ui::utils::dialog_catch_err(
            provider.load_from_data(include_bytes!("../data/style.css")),
            "Could not load style sheet.",
        );
        gtk::StyleContext::add_provider_for_screen(
            &screen,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    let loader = gdk_pixbuf::PixbufLoader::new();
    loader
        .write(include_bytes!(concat!(data_dir!(), "/app.svg")))
        .unwrap_or_else(|e| error!("loader.write() failed: {}", e));
    loader
        .close()
        .unwrap_or_else(|e| error!("loader.close() failed: {}", e));
    if let Some(icon) = loader.get_pixbuf() {
        gtk::Window::set_default_icon(&icon);
    }

    init_actions();
    init_timeouts();

    ui::page_archives::init();
    ui::page_detail::init();
    ui::headerbar::init();
    ui::page_overview::init();
    ui::page_pending::init();
    ui::dialog_info::init();
    ui::app_window::init();

    gtk_app().set_accels_for_action("app.quit", &["<Ctrl>Q"]);
}

fn on_activate(_app: &gtk::Application) {
    debug!("Signal 'activate'");
    app_window::show();
}

fn init_timeouts() {
    glib::timeout_add_local(1000, move || {
        let inhibit_cookie = INHIBIT_COOKIE.get();

        if utils::borg::is_backup_running() {
            if inhibit_cookie.is_none() {
                INHIBIT_COOKIE.update(|c| {
                    *c = Some(gtk_app().inhibit(
                        Some(&main_ui().window()),
                        gtk::ApplicationInhibitFlags::LOGOUT
                            | gtk::ApplicationInhibitFlags::SUSPEND,
                        Some("Backup in Progress"),
                    ))
                });
            }
        } else if let Some(cookie) = inhibit_cookie {
            gtk_app().uninhibit(cookie);
            INHIBIT_COOKIE.update(|c| *c = None);
        }

        Continue(true)
    });
}

async fn quit() -> Result<()> {
    debug!("Running quit routine");
    if utils::borg::is_backup_running() {
        let permission = utils::get_background_permission()
            .await
            .unwrap_or_else(|err| {
                warn!(
                    "background portal: Failed, maybe it's not supported: {:?}",
                    err
                );
                true
            });
        if permission {
            gtk_app().remove_window(&main_ui().window());
            main_ui().window().hide();
        } else {
            ui::utils::confirmation_dialog(
                &gettext("Abort running backup creation?"),
                &gettext("The backup will remain incomplete if aborted now."),
                &gettext("Continue"),
                &gettext("Abort"),
            )
            .await?;
            gtk_app().quit();
        }
    } else {
        gtk_app().quit();
    }

    Ok(())
}

fn init_actions() {
    let action = crate::action::backup_show();
    action.connect_activate(|_, config_id| {
        if let Some(config_id) = config_id.and_then(|v| v.get_str()) {
            app_window::show();
            ui::page_detail::view_backup_conf(&ConfigId::new(config_id.to_string()));
            main_ui().window().present();
        }
    });
    gtk_app().add_action(&action);

    let action = crate::action::backup_start();
    action.connect_activate(|_, config_id| {
        info!("action backup.start: called");
        if let Some(config_id) = config_id.and_then(|v| v.get_str()) {
            ui::page_detail::activate_action_backup(ConfigId::new(config_id.to_string()));
        } else {
            error!("action backup.start: Did not receivce valid config id");
        }
    });
    gtk_app().add_action(&action);

    let action = gio::SimpleAction::new("about", None);
    action.connect_activate(|_, _| ui::dialog_about::show());
    gtk_app().add_action(&action);

    let action = gio::SimpleAction::new("quit", None);
    action.connect_activate(|_, _| {
        debug!("Potential quit: Action app.quit (Ctrl+Q)");
        Handler::run(quit());
    });
    gtk_app().add_action(&action);
}

async fn init_check_borg() -> Result<()> {
    let version_result = utils::spawn_thread("borg::version", borg::version).await?;

    match version_result {
        Err(err) => {
            return Err(Message::new(
                gettext("Failed to run “borg”. Is borg-backup installed correctly?"),
                err,
            )
            .into());
        }
        Ok(version_output) => {
            if let Some(version_string) = version_output.split(' ').nth(1) {
                let version_list = version_string
                    .split('.')
                    .map(str::parse)
                    .map(std::result::Result::ok)
                    .take(2);
                if vec![Some(crate::BORG_MIN_MAJOR), Some(crate::BORG_MIN_MINOR)]
                    .into_iter()
                    .cmp(version_list)
                    == std::cmp::Ordering::Greater
                {
                    return Err(Message::new(
                    gettext("Borg version too old."),
                    gettextf(
                        "The installed version of borg-backup is too old. Some features requiring borg-backup version {}.{} will not work.",
                        &[
                            &crate::BORG_MIN_MAJOR.to_string(),
                            &crate::BORG_MIN_MINOR.to_string(),
                        ],
                    )).into());
                }
            }
        }
    }

    Ok(())
}

fn load_config_e() -> std::io::Result<()> {
    if CONFIG_DIR
        .join(env!("CARGO_PKG_NAME"))
        .join("config.json")
        .is_file()
        && !CONFIG_DIR
            .join(env!("CARGO_PKG_NAME"))
            .join("backup.json")
            .is_file()
    {
        std::fs::rename(
            CONFIG_DIR.join(env!("CARGO_PKG_NAME")).join("config.json"),
            CONFIG_DIR.join(env!("CARGO_PKG_NAME")).join("backup.json"),
        )?;
        let dialog = gtk::MessageDialog::new(
            Some(&main_ui().window()),
            gtk::DialogFlags::MODAL,
            gtk::MessageType::Info,
            gtk::ButtonsType::Ok,
            &gettext("Welcome to Pika Backup version 0.3.\n\nPlease note: The information about the last backup cannot be transferred from the previous version of Pika Backup. It will be available again after the next backup."),
        );
        dialog.connect_response(|dialog, _| dialog.close());
        dialog.show();
    }

    let config = config::Backups::from_default_path()?;
    BACKUP_CONFIG.update(|s| *s = config.clone());

    let history = config::Histories::from_default_path()?;
    BACKUP_HISTORY.update(|s| *s = history.clone());

    Ok(())
}

fn load_config() {
    utils::dialog_catch_err(
        load_config_e(),
        gettext("Could not load configuration file."),
    );
}

fn write_config_e() -> std::io::Result<()> {
    let config: &config::Backups = &BACKUP_CONFIG.load();
    let config_file = tempfile::NamedTempFile::new_in(&*CONFIG_DIR)?;
    serde_json::ser::to_writer_pretty(&config_file, config)?;
    config_file.persist(&config::Backups::default_path()?)?;

    let history: &config::Histories = &BACKUP_HISTORY.load();
    let history_file = tempfile::NamedTempFile::new_in(&*CONFIG_DIR)?;
    serde_json::ser::to_writer_pretty(&history_file, history)?;
    history_file.persist(&config::Histories::default_path()?)?;

    Ok(())
}

fn write_config() -> Result<()> {
    write_config_e().err_to_msg(gettext("Could not write configuration file."))
}
