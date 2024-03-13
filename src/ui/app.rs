use crate::borg;
use crate::config;
use crate::ui;
use crate::ui::prelude::*;
use adw::prelude::*;
use adw::subclass::prelude::*;
use config::TrackChanges;

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
    impl ObjectImpl for App {}
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

            ui::actions::init();
            glib::MainContext::default().spawn_local(async {
                ui::dbus::init().await;
            });

            ui::widget::init();

            // init status tracking
            status_tracking();

            let obj = self.obj();
            obj.set_accels_for_action("app.help", &["F1"]);
            obj.set_accels_for_action("app.quit", &["<Ctrl>Q"]);
            obj.set_accels_for_action("app.setup", &["<Ctrl>N"]);
            obj.set_accels_for_action("app.backup-preferences", &["<Ctrl>comma"]);
            obj.set_accels_for_action("win.show-help-overlay", &["<Ctrl>question"]);

            if BACKUP_CONFIG.load().iter().count() == 1 {
                if let Some(config) = BACKUP_CONFIG.load().iter().next() {
                    main_ui()
                        .page_detail()
                        .backup_page()
                        .view_backup_conf(&config.id);
                }
            }
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
