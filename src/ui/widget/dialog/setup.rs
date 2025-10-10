mod actions;
mod add_existing;
mod advanced_options;
mod ask_password;
mod create_new;
mod encryption;
mod location;
mod location_kind;
mod repo_kind;
mod transfer_option;
mod transfer_prefix;
mod transfer_settings;
mod types;

use adw::prelude::*;
use adw::subclass::prelude::*;

use add_existing::SetupAddExistingPage;
use advanced_options::SetupAdvancedOptionsPage;
use ask_password::SetupAskPasswordPage;
use create_new::SetupCreateNewPage;
pub use encryption::SetupEncryptionPage;
use location::SetupLocationPage;
use location_kind::SetupLocationKindPage;
use transfer_prefix::SetupTransferPrefixPage;
use transfer_settings::SetupTransferSettingsPage;

use crate::ui;
use crate::ui::App;
use crate::ui::prelude::*;
use types::*;

mod imp {
    use adw::subclass::dialog::AdwDialogImplExt;
    use ui::widget::SpinnerPage;

    use crate::config;

    use self::repo_kind::SetupRepoKindPage;

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

        /// This must be set by [Self::set_new_config], otherwise `can-close` will not be up to date
        new_config: RefCell<Option<config::Backup>>,

        /// Whether the config should be saved on close
        save: Cell<bool>,

        /// Indicates that an operation is currently ongoing. Used to prevent multiple input.
        busy: Cell<bool>,

        #[template_child]
        pub(super) navigation_view: TemplateChild<adw::NavigationView>,

        // Initial screen
        #[template_child]
        pub(super) start_page: TemplateChild<SetupRepoKindPage>,

        // Setup the repository location kind (local, remote, specific device)
        #[template_child]
        pub(super) location_kind_page: TemplateChild<SetupLocationKindPage>,

        // First page
        #[template_child]
        pub(super) location_page: TemplateChild<SetupLocationPage>,

        // Encryption page
        #[template_child]
        pub(super) encryption_page: TemplateChild<SetupEncryptionPage>,

        // Add existing repo page
        #[template_child]
        pub(super) add_existing_page: TemplateChild<SetupAddExistingPage>,

        // Ask password page
        #[template_child]
        pub(super) ask_password_page: TemplateChild<SetupAskPasswordPage>,

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
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            SetupAdvancedOptionsPage::ensure_type();

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

            let start_page = self.location_kind_page.clone();
            glib::MainContext::default().spawn_local(async move {
                Handler::handle(start_page.refresh().await);
            });
        }
    }
    impl WidgetImpl for SetupDialog {}
    impl AdwDialogImpl for SetupDialog {
        fn close_attempt(&self) {
            self.parent_close_attempt();

            match self.new_config.take() {
                Some(config) => {
                    // Save the new config
                    let save = self.save.get();
                    let obj = self.obj().clone();
                    Handler::run(async move {
                        let imp = obj.imp();

                        if !save {
                            let dialog = adw::AlertDialog::new(
                                Some(&gettext("Abort Setup?")),
                                Some(&gettext(
                                    "Aborting now will cause the repository to not be added",
                                )),
                            );

                            dialog
                                .add_responses(&[("close", "Continue Setup"), ("abort", "Abort")]);
                            dialog.set_response_appearance(
                                "abort",
                                adw::ResponseAppearance::Destructive,
                            );
                            match &*dialog.choose_future(&obj).await {
                                "abort" => {
                                    obj.force_close();
                                }
                                _ => {
                                    obj.imp().new_config.replace(Some(config));
                                }
                            }

                            return Ok(());
                        }

                        imp.handle_result(imp.save_backup_config(&config).await);

                        obj.force_close();

                        let window = App::default().main_window();

                        // Display a newly added backup in the main window if successful
                        window.view_backup_conf(&config.id);

                        window.announce(
                            // Translators: Announced to accessibility devices when setup dialog has finished
                            &gettext("Backup Repository Added Successfully"),
                            gtk::AccessibleAnnouncementPriority::Medium,
                        );

                        Ok(())
                    })
                }
                _ => {
                    self.obj().force_close();
                }
            };
        }
    }

    #[gtk::template_callbacks]
    impl SetupDialog {
        fn new_config(&self) -> Option<config::Backup> {
            self.new_config.borrow().clone()
        }

        fn set_new_config(&self, config: Option<config::Backup>) {
            self.obj().set_can_close(config.is_none());
            self.new_config.replace(config);
        }

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
        fn on_repo_kind_page_continue(&self, action: SetupAction) {
            self.action.set(action);
            self.location_kind_page.set_repo_action(action);
            self.navigation_view.push(&*self.location_kind_page);
        }

        #[template_callback]
        async fn on_location_kind_page_continue(
            &self,
            action: SetupAction,
            repo: SetupLocationKind,
            file: Option<gio::File>,
        ) {
            if self.busy.replace(true) {
                return;
            }

            self.action.set(action);
            self.obj().notify_action();
            self.location_page.configure(action, repo, file.clone());

            match (action, repo, file) {
                (SetupAction::Init, _, _) => {}
                (SetupAction::AddExisting, SetupLocationKind::Local, file) => {
                    let file = if let Some(file) = file {
                        file
                    } else {
                        match self
                            .handle_result(self.show_add_existing_file_chooser().await)
                            .flatten()
                        {
                            Some(file) => file,
                            _ => {
                                self.busy.set(false);
                                return;
                            }
                        }
                    };

                    let location = SetupRepoLocation::from_file(file);
                    self.location.replace(Some(location.clone()));
                    self.command_line_args.take();

                    let Some(repo) = self.handle_result(
                        self.create_repo_config(location, SetupCommandLineArgs::NONE)
                            .await,
                    ) else {
                        self.busy.set(false);
                        return;
                    };

                    self.show_add_existing_repo_page(repo, None);
                    return;
                }
                (SetupAction::AddExisting, SetupLocationKind::Remote, _) => {}
            }

            // Next page is location page
            self.show_location_page();
            self.busy.set(false);
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
            if self.busy.replace(true) {
                return;
            }

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
                        self.show_add_existing_repo_page(repo, None);
                    }
                }
            }

            self.busy.set(false);
        }

        // Encryption page

        fn show_encryption_page(&self) {
            self.navigation_view.push(&*self.encryption_page);
        }

        #[template_callback]
        async fn on_encryption_page_continue(&self, password: Option<config::Password>) {
            if self.busy.replace(true) {
                return;
            }

            let Some(repo) = self.repo_config.borrow().clone() else {
                error!("Encryption page create button clicked but no repo config set");
                self.navigation_view.pop_to_page(&*self.location_kind_page);

                self.busy.set(false);
                return;
            };

            if self
                .handle_result(self.show_create_new_page(repo, password).await)
                .is_none()
            {
                // An error occurred
                self.navigation_view.pop_to_page(&*self.location_page);
            }

            self.busy.set(false);
        }

        // Add existing page

        fn show_add_existing_repo_page(
            &self,
            repo: config::Repository,
            password: Option<config::Password>,
        ) {
            self.navigation_view.push(&*self.add_existing_page);

            glib::spawn_future_local(glib::clone!(
                #[strong(rename_to = imp)]
                self.ref_counted(),
                async move {
                    imp.add_existing_repo(repo, password).await;
                }
            ));
        }

        async fn add_existing_repo(
            &self,
            repo: config::Repository,
            password: Option<config::Password>,
        ) {
            let result = self
                .add_existing_page
                .check_repo(repo, password.clone())
                .await;

            self.busy.set(false);

            match result {
                Ok(config) => {
                    self.set_new_config(Some(config.clone()));

                    if let Some(password) = &password {
                        self.handle_result(self.save_password(&config, password).await);
                    }

                    self.show_transfer_settings(config).await;
                }
                Err(actions::ConnectRepoError::PasswordWrong) => {
                    self.show_ask_password_page();
                }
                Err(actions::ConnectRepoError::Error(ui::error::Combined::Ui(
                    ui::error::Error::UserCanceled,
                ))) => {
                    self.on_add_existing_page_error(None).await;
                }
                Err(actions::ConnectRepoError::Error(err)) => {
                    self.on_add_existing_page_error(Some(&err.to_string()))
                        .await;
                }
            }
        }

        /// Something went wrong when trying to access the repository.
        ///
        /// Returns us back to the location / start page to allow the user to reconfigure the repository
        async fn on_add_existing_page_error(&self, error: Option<&str>) {
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
                self.navigation_view.pop_to_page(&*self.location_kind_page);
            }

            if let Some(error) = error {
                let error = crate::ui::error::Message::new(
                    gettext("Failed to Configure Repository"),
                    error,
                );
                error.show_transient_for(&*self.obj()).await;
            }
        }

        // Ask Password

        /// Ask for the password when adding an existing repo
        ///
        /// We don't want the add page in the stack here
        fn show_ask_password_page(&self) {
            self.navigation_view.set_animate_transitions(false);
            self.navigation_view.pop();
            self.navigation_view.push(&*self.ask_password_page);
            self.navigation_view.set_animate_transitions(false);
        }

        #[template_callback]
        fn on_ask_password_page_continue(&self, password: config::Password) {
            self.navigation_view.set_animate_transitions(false);
            self.navigation_view.pop();
            self.navigation_view.push(&*self.add_existing_page);
            self.navigation_view.set_animate_transitions(false);

            if let Some(repo) = self.repo_config.borrow().clone() {
                glib::spawn_future_local(glib::clone!(
                    #[strong(rename_to = imp)]
                    self.ref_counted(),
                    #[strong]
                    repo,
                    async move { imp.add_existing_repo(repo, Some(password)).await }
                ));
            }
        }

        // Transfer settings

        /// Shows a page that allows to select includes / excludes from the last archives
        async fn show_transfer_settings(&self, config: config::Backup) {
            let Some(archives) = self.handle_result(
                actions::fetch_archive_list(&config)
                    .await
                    .into_message(gettext("Failed")),
            ) else {
                self.finish();
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
        async fn on_transfer_settings_continue(&self, archive_params: Option<&ArchiveParams>) {
            /*if self.busy.replace(true) {
                return;
            }*/

            let config = self.new_config();
            if let (Some(mut config), Some(archive_params)) = (config, archive_params) {
                let res = actions::transfer_settings(&mut config, archive_params).await;
                self.set_new_config(Some(config));

                if let Some(prefix) = self.handle_result(res) {
                    self.show_transfer_prefix_page(&prefix);
                    self.busy.set(false);
                    return;
                }
            }

            self.busy.set(false);
            self.finish();
        }

        // Transfer prefix

        fn show_transfer_prefix_page(&self, prefix: &config::ArchivePrefix) {
            self.transfer_prefix_page.set_prefix(prefix);
            self.navigation_view.push(&*self.transfer_prefix_page);
        }

        #[template_callback]
        async fn on_transfer_prefix_continue(&self, prefix: &config::ArchivePrefix) {
            let config = self.new_config();

            if let Some(mut config) = config {
                let res = config
                    .set_archive_prefix(prefix.clone(), BACKUP_CONFIG.load().iter())
                    .err_to_msg(gettext("Invalid Archive Prefix"));
                self.set_new_config(Some(config));
                if self.handle_result(res).is_some() {
                    // Setup finished
                    self.finish();
                }
            }
        }

        /// Save the config and password, then close the dialog and show the new backup config
        fn finish(&self) {
            // The config is saved in the close handler
            self.save.set(true);
            self.obj().close();
        }

        fn handle_result<T>(&self, result: Result<T>) -> Option<T> {
            match result {
                Ok(res) => Some(res),
                Err(err) => {
                    let obj = self.obj().clone();
                    glib::spawn_future_local(async move { err.show_transient_for(&obj).await });
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
            self.set_new_config(Some(config.clone()));

            if let Some(password) = password {
                self.handle_result(self.save_password(&config, &password).await);
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
        async fn save_backup_config(&self, config: &crate::config::Backup) -> Result<()> {
            // We shouldn't fail this method after this point, otherwise we
            // leave a half-configured backup config
            BACKUP_CONFIG
                .try_update(glib::clone!(
                    #[strong]
                    config,
                    move |s| {
                        s.insert(config.clone())?;
                        Ok(())
                    }
                ))
                .await?;

            Ok(())
        }

        async fn save_password(
            &self,
            config: &crate::config::Backup,
            password: &config::Password,
        ) -> Result<()> {
            if let Err(err) = ui::utils::password_storage::store_password(config, password).await {
                // Error when storing the password.
                // We don't fail the process here. Sometimes the keyring is just broken and people
                // still want to access their backup archives.
                err.show_transient_for(&*self.obj()).await;
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

            if &visible_page == self.location_kind_page.upcast_ref::<DialogPage>() {
                self.encryption_page.reset();
                self.action.take();
                self.location.take();
                self.command_line_args.take();
                self.set_new_config(None);
            }
        }

        #[template_callback]
        fn on_popped(&self, _page: &adw::NavigationPage) {
            let visible = self.navigation_view.visible_page();
            if visible.and_downcast_ref::<SpinnerPage>().is_some() {
                // Don't pop back to spinner pages
                self.navigation_view.pop();
            }
        }
    }
}

glib::wrapper! {
    pub struct SetupDialog(ObjectSubclass<imp::SetupDialog>)
    @extends gtk::Widget, adw::Dialog,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SetupDialog {
    pub fn new() -> Self {
        glib::Object::new()
    }
}
