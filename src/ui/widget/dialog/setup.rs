mod actions;
mod add_existing;
mod create_new;
mod encryption;
pub mod folder_button;
mod location;
mod start;
mod transfer_option;
mod transfer_prefix;
mod transfer_settings;
mod types;

use adw::prelude::*;
use adw::subclass::prelude::*;

use add_existing::SetupAddExistingPage;
use create_new::SetupCreateNewPage;
use encryption::SetupEncryptionPage;
use location::SetupLocationPage;
use start::SetupStartPage;
use transfer_prefix::SetupTransferPrefixPage;
use transfer_settings::SetupTransferSettingsPage;

use crate::ui;
use crate::ui::prelude::*;
use crate::ui::App;
use types::*;

mod imp {
    use crate::config;

    use crate::borg;

    use super::*;
    use std::cell::{Cell, RefCell};

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
        pub(super) create_new_page: TemplateChild<SetupCreateNewPage>,
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
            if let Some(config) = self.new_config.take() {
                // Save the new config
                self.handle_result(self.save_backup_config(&config));
                self.obj().close();

                // Display it in the UI
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

                    self.show_add_existing_repo_page(repo);
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
                        self.show_add_existing_repo_page(repo);
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

            self.handle_result(self.show_create_new_page(repo, password).await);
        }

        // Add existing page

        fn show_add_existing_repo_page(&self, repo: config::Repository) {
            self.navigation_view.push(&*self.add_existing_page);
            self.add_existing_page.check_and_add_repo(repo);
        }

        #[template_callback]
        async fn on_add_existing_page_continue(
            &self,
            config: config::Backup,
            password: Option<config::Password>,
        ) {
            self.new_config.replace(Some(config.clone()));

            if let Some(password) = password {
                self.handle_result(self.save_password(&config, password).await);
            }

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

        /// Shows a page that allows to select includes / excludes from the last archives
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
            let config = self.new_config.borrow().clone();
            if let Some(mut config) = config {
                let res = actions::transfer_settings(&mut config, archive_params);
                self.new_config.replace(Some(config));

                if let Some(prefix) = self.handle_result(res) {
                    self.show_transfer_prefix_page(&prefix);
                    return;
                }
            }

            self.finish();
        }

        // Transfer prefix

        fn show_transfer_prefix_page(&self, prefix: &config::ArchivePrefix) {
            self.transfer_prefix_page.set_prefix(prefix);
            self.navigation_view.push(&*self.transfer_prefix_page);
        }

        #[template_callback]
        async fn on_transfer_prefix_continue(&self, prefix: &config::ArchivePrefix) {
            let config = self.new_config.borrow().clone();

            if let Some(mut config) = config {
                let res = config
                    .set_archive_prefix(prefix.clone(), BACKUP_CONFIG.load().iter())
                    .err_to_msg(gettext("Invalid Archive Prefix"));
                self.new_config.replace(Some(config));
                self.handle_result(res);
            }

            // Setup finished
            self.finish();
        }

        /// Save the config and password, then close the dialog and show the new backup config
        fn finish(&self) {
            // The config is saved in the close handler
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

        async fn show_create_new_page(
            &self,
            repo: config::Repository,
            password: Option<config::Password>,
        ) -> Result<()> {
            self.repo_config.take();

            self.navigation_view.push(&*self.create_new_page);
            let config = actions::init_new_backup_repo(repo, &password).await?;
            self.new_config.replace(Some(config.clone()));

            if let Some(password) = password {
                self.handle_result(self.save_password(&config, password).await);
            }

            // Everything is done, we have a new repo
            self.finish();
            Ok(())
        }

        /// Create a repo config from an entered location
        async fn create_repo_config(
            &self,
            location: SetupRepoLocation,
            args: SetupCommandLineArgs,
        ) -> Result<config::Repository> {
            self.repo_config.take();
            let repo = actions::create_repo_config(self.action.get(), location, args).await?;
            self.repo_config.replace(Some(repo.clone()));
            Ok(repo)
        }

        /// Add the backup config
        fn save_backup_config(&self, config: &crate::config::Backup) -> Result<()> {
            // We shouldn't fail this method after this point, otherwise we
            // leave a half-configured backup config
            BACKUP_CONFIG.try_update(glib::clone!(@strong config => move |s| {
                s.insert(config.clone())?;
                Ok(())
            }))?;

            Ok(())
        }

        async fn save_password(
            &self,
            config: &crate::config::Backup,
            password: config::Password,
        ) -> Result<()> {
            if let Err(err) = ui::utils::password_storage::store_password(config, &password).await {
                // Error when storing the password.
                // We don't fail the process here. Sometimes the keyring is just broken and people
                // still want to access their backup archives.
                err.show_transient_for(self.obj().root().and_downcast_ref::<gtk::Window>())
                    .await;
            }

            Ok(())
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
