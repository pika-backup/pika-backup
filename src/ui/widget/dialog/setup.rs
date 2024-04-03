pub mod add_task;
mod display;
pub mod folder_button;
mod insert;
mod location;
mod start;
mod transfer_option;
mod util;

use adw::prelude::*;
use adw::subclass::prelude::*;

use location::SetupLocationPage;
use start::SetupStartPage;

use crate::ui;
use crate::ui::error::HandleError;
use crate::ui::prelude::*;
use crate::ui::widget::EncryptionPreferencesGroup;
use add_task::AddConfigTask;
use util::*;

mod imp {
    use crate::config;

    use self::ui::App;

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
        pub(super) page_setup_encryption: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub(super) encryption_preferences_group: TemplateChild<EncryptionPreferencesGroup>,
        #[template_child]
        pub(super) init_button: TemplateChild<gtk::Button>,

        // Ask for password page
        #[template_child]
        pub(super) page_password: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub(super) page_password_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) page_password_pending: TemplateChild<gtk::WindowHandle>,
        #[template_child]
        pub(super) pending_spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub(super) page_password_input: TemplateChild<adw::ToolbarView>,
        #[template_child]
        pub(super) ask_password: TemplateChild<gtk::PasswordEntry>,
        #[template_child]
        pub(super) page_password_continue: TemplateChild<gtk::Button>,

        // Transfer settings page
        #[template_child]
        pub(super) page_transfer: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub(super) page_transfer_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) page_transfer_pending: TemplateChild<adw::ToolbarView>,
        #[template_child]
        pub(super) transfer_pending_spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub(super) page_transfer_select: TemplateChild<adw::ToolbarView>,
        #[template_child]
        pub(super) transfer_suggestions: TemplateChild<gtk::ListBox>,

        // Transfer prefix page
        #[template_child]
        pub(super) page_transfer_prefix: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub(super) prefix: TemplateChild<gtk::Entry>,
        #[template_child]
        pub(super) prefix_submit: TemplateChild<gtk::Button>,

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
            self.page_detail_continue
                .connect_map(clone!(@weak dialog => move |x| dialog.set_default_widget(Some(x))));
            self.init_button
                .connect_map(clone!(@weak dialog => move |x| dialog.set_default_widget(Some(x))));
            self.add_button
                .connect_map(clone!(@weak dialog => move |x| dialog.set_default_widget(Some(x))));
            self.prefix_submit
                .connect_map(clone!(@weak dialog => move |x| dialog.set_default_widget(Some(x))));
            self.page_password_continue
                .connect_map(clone!(@weak dialog => move |x| dialog.set_default_widget(Some(x))));*/

            // Page Overview

            // Page Detail

            // Page Password

            /*
            self.page_password_continue.connect_clicked(
                clone!(@weak imp => move |_| run(async move { imp.event_page_password_continue().await })),
            );

            self.page_password_stack.connect_visible_child_notify(
                clone!(@weak imp => move |_| imp.event_navigation_view_changed()),
            );
            */

            self.pending_spinner.connect_map(|s| s.start());
            self.pending_spinner.connect_unmap(|s| s.stop());

            self.transfer_pending_spinner.connect_map(|s| s.start());
            self.transfer_pending_spinner.connect_unmap(|s| s.stop());

            self.creating_repository_spinner.connect_map(|s| s.start());
            self.creating_repository_spinner.connect_unmap(|s| s.stop());

            let start_page = self.start_page.clone();
            glib::MainContext::default().spawn_local(async move {
                Handler::handle(start_page.refresh().await);
            });
        }
    }
    impl WidgetImpl for SetupDialog {}
    impl WindowImpl for SetupDialog {}
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

        fn show_location_page(&self) {
            self.encryption_preferences_group.reset(true);
            self.ask_password.set_text("");
            self.navigation_view.push(&*self.location_page);
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
                        .show_add_existing_file_chooser()
                        .await
                        .handle_transient_for(&*self.obj())
                        .await
                        .flatten()
                    {
                        file
                    } else {
                        return;
                    };

                    let location = SetupRepoLocation::from_file(file);
                    self.location.replace(Some(location.clone()));
                    self.command_line_args.take();

                    let Some(repo) = self
                        .create_repo_config(location, SetupCommandLineArgs::NONE)
                        .await
                        .handle_transient_for(&*self.obj())
                        .await
                    else {
                        return;
                    };

                    let result = self.action_add_existing_repo(repo).await;

                    if !matches!(result, Err(Error::UserCanceled) | Ok(())) {
                        result.handle_transient_for(&*self.obj()).await;

                        // Go back to start
                        // TODO: The spinner shouldn't be an actual page but just a stack in the entire dialog,
                        // with a cancel button
                        self.navigation_view.pop_to_page(&*self.start_page);
                    }

                    return;
                }
                (SetupAction::AddExisting, SetupLocationKind::Remote, _) => {}
            }

            // Next page is location page
            self.show_location_page();
        }

        #[template_callback]
        async fn on_location_page_continue(
            &self,
            location: SetupRepoLocation,
            args: SetupCommandLineArgs,
        ) {
            let action = self.action.get();

            if let Some(repo) = self
                .create_repo_config(location, args)
                .await
                .handle_transient_for(self.obj().root().and_downcast_ref::<gtk::Window>())
                .await
            {
                self.repo_config.replace(Some(repo.clone()));

                match action {
                    SetupAction::Init => {
                        // Show encryption page before doing anything with the repo config
                        self.navigation_view.push(&*self.page_setup_encryption);
                    }
                    SetupAction::AddExisting => {
                        // Try to access the repository
                        self.action_add_existing_repo(repo)
                            .await
                            .handle_transient_for(
                                self.obj().root().and_downcast_ref::<gtk::Window>(),
                            )
                            .await;
                    }
                }
            }
        }

        #[template_callback]
        async fn on_init_clicked(&self) {
            // TODO temp
            self.on_encryption_page_continue().await;
        }

        #[template_callback]
        async fn on_encryption_page_continue(&self) {
            let Some(repo) = self.repo_config.borrow().clone() else {
                error!("Encryption page create button clicked but no repo config set");
                self.navigation_view.pop_to_page(&*self.start_page);

                return;
            };

            self.action_init_repo(repo)
                .await
                .handle_transient_for(self.obj().root().and_downcast_ref::<gtk::Window>())
                .await;
        }

        async fn action_init_repo(&self, repo: config::Repository) -> Result<()> {
            self.repo_config.take();

            self.navigation_view.push(&*self.page_creating);
            let config = self.init_repo(repo).await?;

            // Everything is done, we have a new repo
            App::default().main_window().view_backup_conf(&config);
            self.obj().close();
            Ok(())
        }

        async fn action_add_existing_repo(&self, repo: config::Repository) -> Result<()> {
            self.ask_password.set_text("");

            self.navigation_view.push(&*self.page_password);
            self.add_task.set_repo(Some(repo.clone()));

            let config = self.add().await?;

            // Everything is done, we added the repo
            App::default().main_window().view_backup_conf(&config);
            self.obj().close();
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
            let Some(visible_page) = self.navigation_view.visible_page().clone() else {
                return;
            };

            if &visible_page == self.start_page.upcast_ref::<adw::NavigationPage>() {
                self.ask_password.set_text("");
                self.encryption_preferences_group.reset(true);

                self.action.take();
                self.location.take();
                self.command_line_args.take();
            }

            if &visible_page == self.location_page.upcast_ref::<adw::NavigationPage>() {
                self.ask_password.set_text("");
            }

            self.ask_password.grab_focus();
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
