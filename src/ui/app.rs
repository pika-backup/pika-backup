use crate::borg;
use crate::config;
use crate::ui;
use crate::ui::prelude::*;
use crate::ui::utils;
use adw::prelude::*;
use adw::subclass::prelude::*;
use config::TrackChanges;

use super::shell;
use super::widget::AppWindow;

mod imp {
    use std::cell::Cell;

    use glib::WeakRef;

    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::App)]
    pub struct App {
        main_window: WeakRef<AppWindow>,
        /// Is the app currently shutting down
        #[property(get)]
        in_shutdown: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for App {
        const NAME: &'static str = "PkApp";
        type Type = super::App;
        type ParentType = adw::Application;
    }

    #[glib::derived_properties]
    impl ObjectImpl for App {
        fn constructed(&self) {
            debug!("App::constructed");
            self.parent_constructed();

            ui::widget::init();
        }
    }
    impl ApplicationImpl for App {
        fn startup(&self) {
            debug!("App::startup");
            self.parent_startup();

            ui::utils::config_io::load_config();
            config::ScheduleStatus::update_on_change(&SCHEDULE_STATUS, |err| {
                Err::<(), std::io::Error>(err).handle("Failed to load Schedule Status")
            })
            .handle("Failed to Load Schedule Status");

            // Force adwaita icon theme
            if let Some(settings) = gtk::Settings::default() {
                settings.set_property("gtk-icon-theme-name", "Adwaita");
            }

            self.setup_actions();
            self.setup_accels();

            glib::MainContext::default().spawn_local(async {
                ui::dbus::init().await;
            });

            // init status tracking
            status_tracking();
        }

        fn activate(&self) {
            debug!("App::activate");
            self.parent_activate();

            let window = self.main_window();
            window.present();
        }

        fn shutdown(&self) {
            debug!("App::shutdown");
            self.parent_shutdown();

            self.in_shutdown.set(true);
            self.obj().notify_in_shutdown();

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

            debug!("App::shutdown finished");
        }
    }
    impl GtkApplicationImpl for App {}
    impl AdwApplicationImpl for App {}

    impl App {
        pub(super) fn main_window(&self) -> AppWindow {
            if let Some(window) = self.main_window.upgrade() {
                window
            } else {
                let window = AppWindow::new(&self.obj());
                self.main_window.set(Some(&window));
                window
            }
        }

        fn setup_actions(&self) {
            let app = self.obj();

            let action = crate::action::backup_show();
            action.connect_activate(glib::clone!(@weak app => move |_, config_id| {
                if let Some(config_id) = config_id.and_then(|v| v.str()) {
                    app.main_window()
                        .view_backup_conf(&ConfigId::new(config_id.to_string()));
                    app.activate();
                }
            }));
            app.add_action(&action);

            let action = crate::action::backup_start();
            action.connect_activate(glib::clone!(@weak app => move |_, config_id| {
                info!("action backup.start: called");
                if let Some(config_id) = config_id.and_then(|v| v.str()).map(ToString::to_string) {
                    let guard = QuitGuard::default();
                    app.main_window().page_detail().backup_page().start_backup(
                        ConfigId::new(config_id),
                        None,
                        guard,
                    );
                } else {
                    error!("action backup.start: Did not receive valid config id");
                }
            }));
            app.add_action(&action);

            let action = gio::SimpleAction::new("about", None);
            action.connect_activate(glib::clone!(@weak app => move |_, _| {
                let dialog = ui::about::window();
                dialog.set_transient_for(Some(&app.main_window()));
                dialog.present()
            }));
            app.add_action(&action);

            let action = gio::SimpleAction::new("setup", None);
            action.connect_activate(|_, _| ui::dialog_setup::show());
            app.add_action(&action);

            let action = gio::SimpleAction::new("help", None);
            let context = app
                .active_window()
                .map(|w| gtk::prelude::WidgetExt::display(&w).app_launch_context());

            action.connect_activate(move |_, _| {
                if let Err(err) =
                    gio::AppInfo::launch_default_for_uri("help:pika-backup", context.as_ref())
                {
                    error!("Launch help error: {err}");
                }
            });
            app.add_action(&action);

            let action = gio::SimpleAction::new("quit", None);
            action.connect_activate(glib::clone!(@weak app => move |_, _| {
                debug!("Potential quit: Action app.quit (Ctrl+Q)");
                Handler::run(async move { app.try_quit().await });
            }));
            app.add_action(&action);

            let action = gio::SimpleAction::new("backup-preferences", None);
            action.connect_activate(glib::clone!(@weak app => move |_, _| {
                if let Some(id) = &**ui::ACTIVE_BACKUP_ID.load() {
                    if app.main_window().page_detail().is_visible() {
                        // Only display when the backup detail page is open
                        ui::dialog_preferences::DialogPreferences::new(id.clone()).present();
                    }
                }
            }));
            app.add_action(&action);

            let action = gio::SimpleAction::new("remove", None);
            action.connect_activate(glib::clone!(@weak app => move |_, _| {
                app.main_window().page_overview().remove_backup()}
            ));
            app.add_action(&action);
        }

        fn setup_accels(&self) {
            let app = self.obj();
            app.set_accels_for_action("app.help", &["F1"]);
            app.set_accels_for_action("app.quit", &["<Ctrl>Q"]);
            app.set_accels_for_action("app.setup", &["<Ctrl>N"]);
            app.set_accels_for_action("app.backup-preferences", &["<Ctrl>comma"]);
            app.set_accels_for_action("win.show-help-overlay", &["<Ctrl>question"]);
        }
    }
}

glib::wrapper! {
    pub struct App(ObjectSubclass<imp::App>)
    @extends adw::Application, gtk::Application, gio::Application,
    @implements gio::ActionMap, gio::ActionGroup;
}

impl App {
    pub fn new() -> Self {
        debug!("Setting up application with id '{}'", crate::APP_ID);
        glib::Object::builder()
            .property("application-id", Some(crate::APP_ID))
            .build()
    }

    pub fn main_window(&self) -> AppWindow {
        self.imp().main_window()
    }

    pub async fn try_quit(&self) -> Result<()> {
        debug!("App::try_quit");
        if utils::borg::is_borg_operation_running() {
            if self.main_window().is_visible() {
                let permission = utils::background_permission().await;

                match permission {
                    Ok(()) => {
                        debug!("Hiding main window as backup is currently running");
                        self.main_window().set_visible(false);
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
                        self.quit_real().await;
                    }
                }
            } else {
                // Someone wants to quit the app from the shell (eg via backgrounds app list)
                // Or we do something wrong and called this erroneously
                debug!("Received quit request while a backup operation is running. Ignoring");
                let notification =
                    gio::Notification::new(&gettext("A Backup Operation is Running"));
                notification.set_body(Some(&gettext(
                    "Pika Backup cannot be quit during a backup operation.",
                )));

                self.send_notification(None, &notification);
            }
        } else {
            self.quit_real().await;
        }

        Ok(())
    }

    async fn quit_real(&self) {
        shell::set_status_message(&gettext("Quit")).await;

        debug!("gio::Application::quit");
        gio::Application::quit(self.upcast_ref());
    }
}

impl std::default::Default for App {
    fn default() -> Self {
        assert!(
            gtk::is_initialized_main_thread(),
            "Calling gio::Application::default from non-main thread"
        );

        gio::Application::default()
            .expect("Application not initialized")
            .downcast::<App>()
            .expect("Application is wrong subclass")
    }
}
