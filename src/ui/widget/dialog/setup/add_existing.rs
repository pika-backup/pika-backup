use crate::borg;
use crate::config;
use crate::ui;
use crate::ui::prelude::*;
use adw::prelude::*;
use adw::subclass::prelude::*;

enum AddRepoError {
    PasswordWrong,
    Error(crate::ui::error::Combined),
}

mod imp {
    use glib::subclass::Signal;

    use crate::ui::widget::dialog_page::PkDialogPageImpl;

    use super::*;
    use std::{cell::RefCell, sync::OnceLock};

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(file = "add_existing.ui")]
    pub struct SetupAddExistingPage {
        password: RefCell<Option<config::Password>>,
        pub(super) repo: RefCell<Option<config::Repository>>,

        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) pending_page: TemplateChild<gtk::WindowHandle>,
        #[template_child]
        pub(super) pending_spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub(super) password_page: TemplateChild<adw::ToolbarView>,
        #[template_child]
        pub(super) password_entry: TemplateChild<gtk::PasswordEntry>,
        #[template_child]
        pub(super) continue_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SetupAddExistingPage {
        const NAME: &'static str = "PkSetupAddExistingPage";
        type Type = super::SetupAddExistingPage;
        type ParentType = DialogPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SetupAddExistingPage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("continue")
                        .param_types([config::Backup::static_type()])
                        .build(),
                    Signal::builder("error")
                        .param_types([String::static_type()])
                        .build(),
                ]
            })
        }
    }
    impl WidgetImpl for SetupAddExistingPage {
        fn map(&self) {
            self.parent_map();
            self.pending_spinner.start();
        }

        fn unmap(&self) {
            self.parent_unmap();
            self.pending_spinner.stop();
        }
    }
    impl NavigationPageImpl for SetupAddExistingPage {}
    impl PkDialogPageImpl for SetupAddExistingPage {}

    #[gtk::template_callbacks]
    impl SetupAddExistingPage {
        fn emit_continue(&self, config: config::Backup) {
            self.obj().emit_by_name::<()>("continue", &[&config]);
        }

        fn emit_error(&self, msg: &str) {
            self.obj().emit_by_name::<()>("error", &[&msg]);
        }

        pub(super) async fn check_and_add_repo(&self, repo: crate::config::Repository) {
            self.stack.set_visible_child(&*self.pending_page);
            self.obj().set_can_pop(false);

            let result = self.try_fetch_archive_list(repo.clone()).await;

            match result {
                Ok(info) => {
                    let config = match self.add_backup_config(repo, info).await {
                        Ok(config) => config,
                        Err(err) => {
                            self.emit_error(&err.to_string());
                            return;
                        }
                    };

                    self.emit_continue(config);
                }
                Err(AddRepoError::PasswordWrong) => {
                    self.stack.set_visible_child(&*self.password_page);
                    self.obj().set_can_pop(true);
                    self.password_entry.grab_focus();
                }
                Err(AddRepoError::Error(err)) => {
                    self.emit_error(&err.to_string());
                }
            };
        }

        #[template_callback]
        async fn on_continue_button(&self) {
            let repo = self.repo.borrow().clone();
            if let Some(repo) = repo {
                self.check_and_add_repo(repo).await;
            }
        }

        /// Validate the password of the repository and try to fetch an archive list.
        pub(super) async fn try_fetch_archive_list(
            &self,
            repo: config::Repository,
        ) -> std::result::Result<borg::List, AddRepoError> {
            // We connect to the repository to validate the password and retrieve its parameters
            let mut borg = borg::CommandOnlyRepo::new(repo.clone());
            borg.password = if self.password_entry.text().is_empty() {
                None
            } else {
                Some(config::Password::new(
                    self.password_entry.text().to_string(),
                ))
            };

            self.password.replace(borg.password.clone());

            let result = ui::utils::borg::exec_repo_only(
                &gettext("Loading Backup Repository"),
                borg,
                |borg| borg.peek(),
            )
            .await;

            match result {
                Ok(info) => Ok(info),
                Err(ui::error::Combined::Borg(borg::Error::Failed(
                    borg::Failure::PassphraseWrong,
                ))) => {
                    // The password was wrong. Let's ask for the password again.
                    Err(AddRepoError::PasswordWrong)
                }
                Err(err) => {
                    // Some other error occurred -> we abort the entire process
                    Err(AddRepoError::Error(err))
                }
            }
        }

        /// Add the backup config
        async fn add_backup_config(
            &self,
            repo: crate::config::Repository,
            info: borg::List,
        ) -> Result<config::Backup> {
            let password = self.password.borrow().clone();
            let config = config::Backup::new(repo, info, password.is_some());

            // We shouldn't fail this method after this point, otherwise we
            // leave a half-configured backup config
            BACKUP_CONFIG.try_update(glib::clone!(@strong config => move |s| {
                s.insert(config.clone())?;
                Ok(())
            }))?;

            if let Some(password) = password {
                if let Err(err) =
                    ui::utils::password_storage::store_password(&config, &password).await
                {
                    // Error when storing the password.
                    // We don't fail the process here. Sometimes the keyring is just broken and people
                    // still want to be able to access their backup archives.
                    err.show_transient_for(self.obj().root().and_downcast_ref::<gtk::Window>())
                        .await;
                }
            }

            Ok(config)
        }
    }
}

glib::wrapper! {
    pub struct SetupAddExistingPage(ObjectSubclass<imp::SetupAddExistingPage>)
    @extends DialogPage, adw::NavigationPage, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SetupAddExistingPage {
    pub fn check_and_add_repo(&self, repo: config::Repository) {
        self.imp().password_entry.set_text("");
        self.imp().repo.replace(Some(repo.clone()));

        let obj = self.clone();
        glib::spawn_future_local(async move {
            obj.imp().check_and_add_repo(repo).await;
        });
    }
}
