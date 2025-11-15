use adw::prelude::*;
use adw::subclass::prelude::*;
use common::config;
use config::TrackChanges;

use super::shell;
use super::widget::setup::SetupDialog;
use super::widget::{AppWindow, PreferencesDialog};
use crate::prelude::*;
use crate::utils;
use crate::widget::UnmountArchives;

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
            tracing::debug!("App::constructed");
            self.parent_constructed();

            crate::widget::init();
        }
    }
    impl ApplicationImpl for App {
        fn startup(&self) {
            tracing::debug!("App::startup");
            self.parent_startup();

            crate::utils::config_io::load_config();

            config::ScheduleStatus::update_on_change(&SCHEDULE_STATUS, |err| {
                Err::<(), std::io::Error>(err).handle("Failed to load Schedule Status")
            })
            .handle("Failed to Load Schedule Status");

            // Force adwaita icon theme
            if let Some(settings) = gtk::Settings::default() {
                settings.set_property("gtk-icon-theme-name", "Adwaita");
            }

            gtk::Window::set_default_icon_name(common::APP_ID);

            self.obj().setup_actions();

            glib::MainContext::default().spawn_local(async {
                crate::dbus::init().await;
                crate::utils::borg::cleanup_repo_mounts().await;
            });

            // init status tracking
            status_tracking();
        }

        fn activate(&self) {
            tracing::debug!("App::activate");
            self.parent_activate();

            let window = self.main_window();
            window.present();
        }

        fn shutdown(&self) {
            tracing::debug!("App::shutdown");
            self.parent_shutdown();

            self.in_shutdown.set(true);
            self.obj().notify_in_shutdown();

            let result = smol::block_on(BACKUP_HISTORY.try_update(|histories| {
                config::Histories::handle_shutdown(histories);
                Ok(())
            }));

            if let Err(err) = result {
                tracing::error!("Failed to write config during shutdown: {}", err);
            }

            tracing::debug!("App::shutdown finished");
        }
    }
    impl GtkApplicationImpl for App {}
    impl AdwApplicationImpl for App {}

    impl App {
        pub(super) fn main_window(&self) -> AppWindow {
            match self.main_window.upgrade() {
                Some(window) => window,
                _ => {
                    let window = AppWindow::new(&self.obj());
                    self.main_window.set(Some(&window));
                    window
                }
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
        tracing::debug!("Setting up application with id '{}'", common::APP_ID);
        glib::Object::builder()
            .property("application-id", Some(common::APP_ID))
            .build()
    }

    pub fn main_window(&self) -> AppWindow {
        self.imp().main_window()
    }

    fn setup_actions(&self) {
        let actions = [
            gio::ActionEntryBuilder::new("about")
                .activate(|app: &Self, _, _| {
                    let dialog = crate::widget::dialog::about::window();
                    dialog.present(Some(&app.main_window()))
                })
                .build(),
            gio::ActionEntryBuilder::new("setup")
                .activate(|app: &Self, _, _| {
                    let window = app.main_window();
                    SetupDialog::new().present(Some(&window));
                })
                .build(),
            gio::ActionEntryBuilder::new("help")
                .activate(|app: &Self, _, _| {
                    let context = app
                        .active_window()
                        .map(|w| gtk::prelude::WidgetExt::display(&w).app_launch_context());

                    if let Err(err) =
                        gio::AppInfo::launch_default_for_uri("help:pika-backup", context.as_ref())
                    {
                        tracing::error!("Launch help error: {err}");
                    }
                })
                .build(),
            gio::ActionEntryBuilder::new("quit")
                .activate(|app: &Self, _, _| {
                    tracing::debug!("Potential quit: Action app.quit (Ctrl+Q)");
                    let app = app.clone();
                    Handler::run(async move { app.try_quit().await });
                })
                .build(),
            gio::ActionEntryBuilder::new("backup-preferences")
                .activate(|app: &Self, _, _| {
                    if let Some(id) = &**crate::ACTIVE_BACKUP_ID.load()
                        && app.main_window().page_detail().is_visible()
                    {
                        // Only display when the backup detail page is open
                        PreferencesDialog::new(id.clone()).present(Some(&app.main_window()));
                    }
                })
                .build(),
            gio::ActionEntryBuilder::new("remove")
                .activate(|app: &Self, _, _| app.main_window().page_overview().remove_backup())
                .build(),
            gio::ActionEntryBuilder::new("backup.start")
                .parameter_type(Some(&String::static_variant_type()))
                .activate(|app: &Self, _, config_id| {
                    tracing::info!("action backup.start: called");
                    if let Some(config_id) =
                        config_id.and_then(|v| v.str()).map(ToString::to_string)
                    {
                        let guard = QuitGuard::default();
                        app.main_window().page_detail().backup_page().start_backup(
                            ConfigId::new(config_id),
                            None,
                            guard,
                        );
                    } else {
                        tracing::error!("action backup.start: Did not receive valid config id");
                    }
                })
                .build(),
            gio::ActionEntryBuilder::new("backup.show")
                .parameter_type(Some(&String::static_variant_type()))
                .activate(|app: &Self, _, config_id| {
                    if let Some(config_id) = config_id.and_then(|v| v.str()) {
                        app.main_window()
                            .view_backup_conf(&ConfigId::new(config_id.to_string()));
                        app.activate();
                    }
                })
                .build(),
        ];

        self.add_action_entries(actions);

        self.set_accels_for_action("app.help", &["F1"]);
        self.set_accels_for_action("app.quit", &["<Ctrl>Q"]);
    }

    pub async fn try_quit(&self) -> Result<()> {
        tracing::debug!("App::try_quit");

        let dialog = UnmountArchives::new();
        dialog.execute(&self.main_window()).await?;

        if BACKUP_HISTORY.load().iter().any(|(_, x)| x.is_browsing()) {
            tracing::debug!("Some archives are still mounted for browsing.");
        } else {
            tracing::debug!("No archives mounted for browsing");
        }

        if utils::borg::is_borg_operation_running() {
            if self.main_window().is_visible() {
                let permission = utils::background_permission().await;
                match permission {
                    Ok(()) => {
                        tracing::debug!("Hiding main window as backup is currently running");
                        self.main_window().set_visible(false);
                    }
                    Err(err) => {
                        err.show().await;

                        crate::utils::confirmation_dialog(
                            &self.main_window(),
                            &gettext("Abort Backup?"),
                            &gettext("The backup will remain incomplete if aborted now"),
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
                tracing::debug!(
                    "Received quit request while a backup operation is running. Ignoring"
                );
                let notification = gio::Notification::new(&gettext("Backup Operation Running"));
                notification.set_body(Some(&gettext(
                    "Pika Backup cannot be quit during a backup operation",
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

        tracing::debug!("gio::Application::quit");
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
