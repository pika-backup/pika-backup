//! User interface

mod app_window;
mod backup_status;
#[allow(dead_code)]
mod builder;
mod dbus;
mod dialog_about;
mod dialog_device_missing;
mod dialog_encryption_password;
mod dialog_info;
mod dialog_prune;
mod dialog_setup;
mod dialog_shortcuts;
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
mod toast_size_estimate;
mod utils;
mod widgets;

pub(crate) use globals::{BACKUP_CONFIG, SCHEDULE_STATUS};

use gtk::prelude::*;

use crate::borg;
use crate::config;
use crate::config::Loadable;
use crate::ui;
use crate::ui::prelude::*;
use config::ArcSwapWriteable;

pub fn main() {
    // suppress "gdk_pixbuf_from_pixdata()" debug spam
    glib::log_set_handler(
        Some("GdkPixbuf"),
        glib::LogLevels::LEVEL_DEBUG,
        false,
        false,
        |_, _, _| {},
    );

    let mut text_domain = gettextrs::TextDomain::new(env!("CARGO_PKG_NAME"));
    if let Some(localedir) = option_env!("LOCALEDIR") {
        text_domain = text_domain.prepend(localedir);
    }

    if let Err(err) = text_domain
        .locale_category(gettextrs::LocaleCategory::LcAll)
        .init()
    {
        info!("TextDomain::init() failed: {}", err);
    }

    gtk_app().connect_startup(on_startup);
    gtk_app().connect_activate(on_activate);
    gtk_app().connect_shutdown(on_shutdown);

    // Ctrl-C handling
    glib::unix_signal_add(nix::sys::signal::Signal::SIGINT as i32, on_ctrlc);

    gtk_app().run();
}

fn on_ctrlc() -> Continue {
    debug!("Quit: SIGINT (Ctrl+C)");

    BORG_OPERATION.with(|operations| {
        for op in operations.load().values() {
            op.set_instruction(borg::Instruction::Abort(borg::Abort::User));
        }
    });

    gtk_app().release();
    Continue(true)
}

fn on_shutdown(app: &gtk::Application) {
    app.mark_busy();
    IS_SHUTDOWN.swap(std::sync::Arc::new(true));
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

fn on_startup(_app: &gtk::Application) {
    adw::init();
    debug!("Signal 'startup'");
    load_config();

    if let Some(display) = gtk::gdk::Display::default() {
        let provider = gtk::CssProvider::new();
        provider.load_from_data(include_bytes!("../data/style.css"));

        gtk::StyleContext::add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    init_actions();
    ui::dbus::init();

    ui::app_window::init();
    ui::headerbar::init();

    ui::page_overview::init();

    ui::page_detail::init();
    ui::page_backup::init::init();
    ui::page_archives::init();
    ui::page_schedule::init::init();

    ui::dialog_info::init();

    gtk_app().set_accels_for_action("app.help", &["F1"]);
    gtk_app().set_accels_for_action("app.quit", &["<Ctrl>Q"]);
    gtk_app().set_accels_for_action("app.setup", &["<Ctrl>N"]);

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
}

fn on_activate(_app: &gtk::Application) {
    debug!("Signal 'activate'");
    app_window::show();
}

async fn quit() -> Result<()> {
    debug!("Running quit routine");
    if utils::borg::is_backup_running() {
        let permission = utils::background_permission().await;

        match permission {
            Ok(()) => {
                gtk_app().remove_window(&main_ui().window());
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
                gtk_app().quit();
            }
        }
    } else {
        gtk_app().quit();
    }

    Ok(())
}

fn init_actions() {
    let action = crate::action::backup_show();
    action.connect_activate(|_, config_id| {
        if let Some(config_id) = config_id.and_then(|v| v.str()) {
            app_window::show();
            ui::page_backup::view_backup_conf(&ConfigId::new(config_id.to_string()));
            main_ui().window().present();
        }
    });
    gtk_app().add_action(&action);

    let action = crate::action::backup_start();
    action.connect_activate(|_, config_id| {
        info!("action backup.start: called");
        if let Some(config_id) = config_id.and_then(|v| v.str()) {
            ui::page_backup::activate_action_backup(ConfigId::new(config_id.to_string()));
        } else {
            error!("action backup.start: Did not receivce valid config id");
        }
    });
    gtk_app().add_action(&action);

    let action = gio::SimpleAction::new("about", None);
    action.connect_activate(|_, _| ui::dialog_about::show());
    gtk_app().add_action(&action);

    let action = gio::SimpleAction::new("shortcuts", None);
    action.connect_activate(|_, _| ui::dialog_shortcuts::show());
    gtk_app().add_action(&action);

    let action = gio::SimpleAction::new("setup", None);
    action.connect_activate(|_, _| ui::dialog_setup::show());
    gtk_app().add_action(&action);

    let action = gio::SimpleAction::new("help", None);
    action.connect_activate(|_, _| {
        gtk::show_uri(
            Some(&main_ui().window()),
            "help:pika-backup",
            gtk::gdk::CURRENT_TIME,
        )
    });
    gtk_app().add_action(&action);

    let action = gio::SimpleAction::new("quit", None);
    action.connect_activate(|_, _| {
        debug!("Potential quit: Action app.quit (Ctrl+Q)");
        Handler::run(quit());
    });
    gtk_app().add_action(&action);

    let action = gio::SimpleAction::new("remove", None);
    action.connect_activate(|_, _| page_overview::remove_backup());
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
                        gettext("Cannot check borg-backup version"),
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

fn load_config_e() -> std::io::Result<()> {
    if glib::user_config_dir()
        .join(env!("CARGO_PKG_NAME"))
        .join("config.json")
        .is_file()
        && !glib::user_config_dir()
            .join(env!("CARGO_PKG_NAME"))
            .join("backup.json")
            .is_file()
    {
        std::fs::rename(
            glib::user_config_dir()
                .join(env!("CARGO_PKG_NAME"))
                .join("config.json"),
            glib::user_config_dir()
                .join(env!("CARGO_PKG_NAME"))
                .join("backup.json"),
        )?;
    }

    BACKUP_CONFIG.swap(Arc::new(config::Writeable::from_file()?));
    BACKUP_CONFIG.update(|backups| {
        let mut new = backups.clone();

        for mut config in new.iter_mut() {
            if config.config_version < config::VERSION {
                config.config_version = config::VERSION;
            }
        }

        *backups = new;
    });
    // potentially write generated default value
    BACKUP_CONFIG.write_file()?;

    BACKUP_HISTORY.swap(Arc::new(config::Histories::from_file_ui()?));
    // potentially write internal error status
    BACKUP_HISTORY.write_file()?;

    Ok(())
}

fn load_config() {
    let res = load_config_e().err_to_msg(gettext("Could not load configuration file."));
    if let Err(err) = res {
        glib::MainContext::default().block_on(err.show());
    }
}

fn write_config_e() -> std::io::Result<()> {
    debug!("Rewriting all configs");

    BACKUP_CONFIG.write_file()?;
    BACKUP_HISTORY.write_file()?;

    Ok(())
}

fn write_config() -> Result<()> {
    write_config_e().err_to_msg(gettext("Could not write configuration file."))
}
