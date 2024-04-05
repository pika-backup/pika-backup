mod add_existing;
pub mod add_task;
mod encryption;
pub mod folder_button;
mod insert;
mod location;
mod start;
mod transfer_option;
mod transfer_prefix;
mod transfer_settings;
mod util;

use adw::prelude::*;
use adw::subclass::prelude::*;

use add_existing::SetupAddExistingPage;
use encryption::SetupEncryptionPage;
use location::SetupLocationPage;
use start::SetupStartPage;
use transfer_settings::SetupTransferSettingsPage;

use crate::ui;
use crate::ui::prelude::*;
use crate::ui::App;
use add_task::AddConfigTask;
use util::*;

mod imp {
    use crate::config;

    use crate::borg;

    use self::transfer_option::ArchiveParams;
    use self::transfer_prefix::SetupTransferPrefixPage;

    use super::*;
    use std::cell::{Cell, RefCell};
    use std::collections::BTreeSet;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[template(file = "setup.ui")]
    #[properties(wrapper_type = super::SetupDialog)]
    pub struct SetupDialog {
        #[property(get, builder(SetupAction::Init))]
        action: Cell<SetupAction>,
        #[property(get)]
        location: RefCell<Option<SetupRepoLocation>>,
        #[property(get)]
        repo_config: RefCell<Option<config::Repository>>,
        #[property(get)]
        command_line_args: RefCell<SetupCommandLineArgs>,
        #[property(get)]
        add_password: RefCell<Option<config::Password>>,
        new_config: RefCell<Option<config::Backup>>,

        #[template_child]
        pub(super) navigation_view: TemplateChild<adw::NavigationView>,

        // Initial screen
        #[template_child]
        pub(super) start_page: TemplateChild<SetupStartPage>,

        // First page
        #[template_child]
        pub(super) location_page: TemplateChild<SetupLocationPage>,

        // Encryption page
        #[template_child]
        pub(super) encryption_page: TemplateChild<SetupEncryptionPage>,

        // Add existing repo page
        #[template_child]
        pub(super) add_existing_page: TemplateChild<SetupAddExistingPage>,

        // Transfer settings page
        #[template_child]
        pub(super) transfer_settings_page: TemplateChild<SetupTransferSettingsPage>,

        // Transfer prefix page
        #[template_child]
        pub(super) transfer_prefix_page: TemplateChild<SetupTransferPrefixPage>,

        // Creating spinner page
        #[template_child]
        pub(super) page_creating: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub(super) creating_repository_spinner: TemplateChild<gtk::Spinner>,

        #[template_child]
        pub(super) add_task: TemplateChild<AddConfigTask>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SetupDialog {
        const NAME: &'static str = "PkSetupDialog";
        type Type = super::SetupDialog;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for SetupDialog {
        fn constructed(&self) {
            self.parent_constructed();

            // Default buttons

            /*
            self.prefix_submit
                .connect_map(clone!(@weak dialog => move |x| dialog.set_default_widget(Some(x))));
            self.page_password_continue
                .connect_map(clone!(@weak dialog => move |x| dialog.set_default_widget(Some(x))));*/

            self.creating_repository_spinner.connect_map(|s| s.start());
            self.creating_repository_spinner.connect_unmap(|s| s.stop());

            let start_page = self.start_page.clone();
            glib::MainContext::default().spawn_local(async move {
                Handler::handle(start_page.refresh().await);
            });
        }
    }
    impl WidgetImpl for SetupDialog {}
    impl WindowImpl for SetupDialog {
        fn close_request(&self) -> glib::Propagation {
            // Display a newly added backup in the main window if successful
            if let Some(config) = &*self.new_config.borrow() {
                App::default().main_window().view_backup_conf(&config.id);
            }

            self.parent_close_request()
        }
    }
    impl AdwWindowImpl for SetupDialog {}

    #[gtk::template_callbacks]
    impl SetupDialog {
        async fn show_add_existing_file_chooser(&self) -> Result<Option<gio::File>> {
            if let Some(path) =
                ui::utils::folder_chooser_dialog(&gettext("Setup Existing Repository"), None)
                    .await
                    .ok()
                    .and_then(|x| x.path())
            {
                self.obj().set_visible(true);
                if ui::utils::is_backup_repo(&path).await {
                    return Ok(Some(gio::File::for_path(path)));
                } else {
                    return Err(Message::new(
                        gettext("Location is not a valid backup repository."),
                        gettext(
                            "The repository must originate from Pika Backup or compatible software.",
                        ),
                    )
                    .into());
                }
            }

            Ok(None)
        }

        #[template_callback]
        async fn on_start_page_continue(
            &self,
            action: SetupAction,
            repo: SetupLocationKind,
            file: Option<gio::File>,
        ) {
            self.action.set(action);
            self.obj().notify_action();
            self.location_page.configure(action, repo, file.clone());

            match (action, repo, file) {
                (SetupAction::Init, _, _) => {}
                (SetupAction::AddExisting, SetupLocationKind::Local, file) => {
                    let file = if let Some(file) = file {
                        file
                    } else if let Some(file) = self
                        .handle_result(self.show_add_existing_file_chooser().await)
                        .flatten()
                    {
                        file
                    } else {
                        return;
                    };

                    let location = SetupRepoLocation::from_file(file);
                    self.location.replace(Some(location.clone()));
                    self.command_line_args.take();

                    let Some(repo) = self.handle_result(
                        self.create_repo_config(location, SetupCommandLineArgs::NONE)
                            .await,
                    ) else {
                        return;
                    };

                    self.show_add_existing_repo(repo);
                    return;
                }
                (SetupAction::AddExisting, SetupLocationKind::Remote, _) => {}
            }

            // Next page is location page
            self.show_location_page();
        }

        // Location page

        fn show_location_page(&self) {
            self.encryption_page.reset();
            self.navigation_view.push(&*self.location_page);
        }

        #[template_callback]
        async fn on_location_page_continue(
            &self,
            location: SetupRepoLocation,
            args: SetupCommandLineArgs,
        ) {
            let action = self.action.get();

            if let Some(repo) = self.handle_result(self.create_repo_config(location, args).await) {
                self.repo_config.replace(Some(repo.clone()));

                match action {
                    SetupAction::Init => {
                        // Show encryption page before doing anything with the repo config
                        self.show_encryption_page();
                    }
                    SetupAction::AddExisting => {
                        // Try to access the repository
                        self.show_add_existing_repo(repo);
                    }
                }
            }
        }

        // Encryption page

        fn show_encryption_page(&self) {
            self.navigation_view.push(&*self.encryption_page);
        }

        #[template_callback]
        async fn on_encryption_page_continue(&self, password: Option<config::Password>) {
            let Some(repo) = self.repo_config.borrow().clone() else {
                error!("Encryption page create button clicked but no repo config set");
                self.navigation_view.pop_to_page(&*self.start_page);

                return;
            };

            self.handle_result(self.action_init_repo(repo, password).await);
        }

        // Add existing page

        fn show_add_existing_repo(&self, repo: config::Repository) {
            self.navigation_view.push(&*self.add_existing_page);
            self.add_existing_page.check_and_add_repo(repo);
        }

        #[template_callback]
        async fn on_add_existing_page_continue(&self, config: config::Backup) {
            self.new_config.replace(Some(config.clone()));
            self.show_transfer_settings(config).await;
        }

        /// Something went wrong when trying to access the repository.
        ///
        /// Returns us back to the location / start page to allow the user to reconfigure the repository
        #[template_callback]
        async fn on_add_existing_page_error(&self, error: &str) {
            // We have two options here: Either we have location page in the stack or we don't,
            // depending on whether we are adding a remote repo from a custom URL or a local one /
            // a remote repo from a preset.
            //
            // So we check whether it's in the stack and return to the appropriate page.
            if self
                .navigation_view
                .navigation_stack()
                .into_iter()
                .any(|res| match res {
                    Ok(page) => &page == self.location_page.upcast_ref::<glib::Object>(),
                    Err(_) => false,
                })
            {
                self.navigation_view.pop_to_page(&*self.location_page);
            } else {
                self.navigation_view.pop_to_page(&*self.start_page);
            }

            let error =
                crate::ui::error::Message::new(gettext("Failed to Configure Repository"), error);
            error
                .show_transient_for(self.obj().root().and_downcast_ref::<gtk::Window>())
                .await;
        }

        // Transfer settings

        async fn show_transfer_settings(&self, config: config::Backup) {
            let guard = QuitGuard::default();
            let mut list_command = borg::Command::<borg::task::List>::new(config.clone());
            list_command.task.set_limit_first(100);

            let Some(archives) = self.handle_result(
                ui::utils::borg::exec(list_command, &guard)
                    .await
                    .into_message(gettext("Failed")),
            ) else {
                return;
            };

            self.transfer_settings_page.set_archives(archives);
            if self.transfer_settings_page.has_suggestions() {
                self.navigation_view.push(&*self.transfer_settings_page);
            } else {
                self.finish();
            }
        }

        #[template_callback]
        async fn on_transfer_settings_continue(&self, archive_params: &ArchiveParams) {
            if let Some(prefix) = self.handle_result(self.transfer_settings(archive_params).await) {
                self.show_transfer_prefix_page(&prefix);
            } else {
                self.finish();
            }
        }

        async fn transfer_settings(
            &self,
            archive_params: &ArchiveParams,
        ) -> Result<config::ArchivePrefix> {
            let Some(config) = self.new_config.borrow().clone() else {
                return Err(Error::UserCanceled);
            };

            let config_id = config.id;
            BACKUP_CONFIG.try_update(enclose!((archive_params, config_id) move |config| {
                let conf = config.try_get_mut(&config_id)?;

                conf.include = archive_params.parsed.include.clone();
                conf.exclude = BTreeSet::from_iter( archive_params.parsed.exclude.clone().into_iter().map(|x| x.into_relative()));

                Ok(())
            }))?;

            let entry = config::history::RunInfo {
                end: archive_params
                    .end
                    .and_local_timezone(chrono::Local)
                    .unwrap(),
                outcome: borg::Outcome::Completed {
                    stats: archive_params.stats.clone(),
                },
                messages: Default::default(),
                include: archive_params.parsed.include.clone(),
                exclude: archive_params.parsed.exclude.clone(),
            };

            // Create fake history entry for duration estimate to be good for first run
            BACKUP_HISTORY.try_update(enclose!((config_id) move |histories| {
                histories.insert(config_id.clone(), entry.clone());
                Ok(())
            }))?;

            let configs = BACKUP_CONFIG.load();
            let config = configs.try_get(&config_id)?;

            let prefix = if let Some(prefix) = &archive_params.prefix {
                prefix.clone()
            } else {
                config.archive_prefix.clone()
            };

            Ok(prefix)
        }

        // Transfer prefix

        fn show_transfer_prefix_page(&self, prefix: &config::ArchivePrefix) {
            self.transfer_prefix_page.set_prefix(prefix);
            self.navigation_view.push(&*self.transfer_prefix_page);
        }

        #[template_callback]
        fn on_transfer_prefix_continue(&self, prefix: &config::ArchivePrefix) {
            let config = self.new_config.borrow().clone();

            if let Some(config) = &config {
                let config_id = config.id.clone();
                self.handle_result(BACKUP_CONFIG.try_update(
                    enclose!((prefix, config_id) move |config| {
                        config
                            .try_get_mut(&config_id)?
                            .set_archive_prefix(
                                prefix.clone(),
                                BACKUP_CONFIG.load().iter(),
                            )
                            .err_to_msg(gettext("Invalid Archive Prefix"))
                    }),
                ));
            }

            // Setup finished
            self.finish();
        }

        /// Close the dialog and show the new backup config
        fn finish(&self) {
            self.obj().close();
        }

        fn handle_result<T>(&self, result: Result<T>) -> Option<T> {
            match result {
                Ok(res) => Some(res),
                Err(err) => {
                    let window = self.obj().root().and_downcast::<gtk::Window>();
                    glib::spawn_future_local(async move {
                        err.show_transient_for(window.as_ref()).await
                    });
                    None
                }
            }
        }

        async fn action_init_repo(
            &self,
            repo: config::Repository,
            password: Option<config::Password>,
        ) -> Result<()> {
            self.repo_config.take();

            self.navigation_view.push(&*self.page_creating);
            let config = self.init_repo(repo, password).await?;

            // Everything is done, we have a new repo
            App::default().main_window().view_backup_conf(&config);
            self.finish();
            Ok(())
        }

        async fn create_repo_config(
            &self,
            location: SetupRepoLocation,
            args: SetupCommandLineArgs,
        ) -> Result<config::Repository> {
            // Create a repo config from an entered location
            self.repo_config.take();

            // A location can either be a borg remote ssh URI or a gio::File
            let mut repo = match location {
                SetupRepoLocation::Remote(url) => {
                    // A remote config can only be verified by running borg and checking if it works
                    debug!("Creating remote repository config with uri: {}", url);
                    config::remote::Repository::from_uri(url).into_config()
                }
                SetupRepoLocation::Local(file) => {
                    // A local repo can be either:
                    //  * Regular file that is not mounted via gvfs
                    //  * GVFS URI
                    let uri = file.uri().to_string();

                    // If we are creating a new repository we need to use the parent directory for
                    // the mount check, because the repo dir does not exist yet
                    let mount_file = if self.action.get() == SetupAction::Init {
                        file.parent().unwrap_or_else(|| file.clone())
                    } else {
                        file.clone()
                    };

                    // Check if the file is contained in a [`gio::Mount`]
                    let mount = mount_file.find_enclosing_mount(Some(&gio::Cancellable::new()));
                    debug!("Find mount for '{}': {:?}", mount_file.uri(), mount);

                    // Check if we have an actual path already
                    let path = if let Some(path) = file.path() {
                        path
                    } else {
                        // We don't. Let's try to mount the URI
                        ui::repo::mount_enclosing(&mount_file).await?;

                        file.path().ok_or_else(|| {
                            warn!(
                                "Finding enclosing mount failed. URI: '{}', mount result: {:?}", uri, mount
                            );
                            Error::Message(Message::new(
                                gettext("Repository location not found."),
                                gettext(
                                    "A mount operation succeeded but the location is still unavailable.",
                                ),
                            ))
                        })?
                    };

                    if let Ok(mount) = mount {
                        // We found a mount
                        debug!(
                            "Creating local repository config with mount: '{}', path: {:?}, uri: {:?}",
                            mount.name(), path, uri
                        );
                        config::local::Repository::from_mount(mount, path, uri).into_config()
                    } else {
                        // We have a path, but we couldn't find a [`gio::Mount`] to go with it.
                        // We resort to just store the path.
                        //
                        // Note: Not storing a mount disables GVFS features, such as detecting drives
                        // that have been renamed, or being able to mount the repository location ourselves.
                        // This is not the best configuration.
                        debug!("Creating local repository config with path: {:?}", path);
                        config::local::Repository::from_path(path).into_config()
                    }
                }
            };

            // Add command line arguments to repository config if given
            let args_vec = args.into_inner();
            if !args_vec.is_empty() {
                repo.set_settings(Some(config::BackupSettings {
                    command_line_args: Some(args_vec),
                }));
            }

            self.repo_config.replace(Some(repo.clone()));
            Ok(repo)
        }

        #[template_callback]
        fn on_visible_page_notify(&self) {
            let Some(visible_page) = self
                .navigation_view
                .visible_page()
                .and_downcast::<DialogPage>()
            else {
                return;
            };

            if &visible_page == self.start_page.upcast_ref::<DialogPage>() {
                self.encryption_page.reset();

                self.action.take();
                self.location.take();
                self.command_line_args.take();
                self.new_config.take();
            }
        }
    }
}

glib::wrapper! {
    pub struct SetupDialog(ObjectSubclass<imp::SetupDialog>)
    @extends gtk::Widget, gtk::Window, adw::Window,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SetupDialog {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn present_with(&self, transient_for: &impl IsA<gtk::Window>) {
        self.set_transient_for(Some(transient_for));
        self.present();
    }
}
