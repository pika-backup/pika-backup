use adw::prelude::*;
use adw::subclass::prelude::*;

use crate::config::BackupSettings;
use crate::ui::prelude::*;

mod imp {
    use crate::{borg, config::UserScriptKind, ui::widget::EncryptionPreferencesGroup};

    use super::*;
    use glib::Properties;
    use std::cell::{Cell, OnceCell, RefCell};

    #[derive(Debug, Default, Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::PreferencesDialog)]
    #[template(file = "preferences.ui")]
    pub struct PreferencesDialog {
        #[property(get, set, construct_only)]
        pub config_id: OnceCell<ConfigId>,

        #[template_child]
        title_pref_group: TemplateChild<adw::PreferencesGroup>,

        #[template_child]
        title_entry: TemplateChild<adw::EntryRow>,

        #[property(get, set)]
        config_title: RefCell<String>,

        close_real: Cell<bool>,

        command_line_args_error: RefCell<Option<crate::ui::error::Error>>,
        pre_backup_command_error: RefCell<Option<crate::ui::error::Error>>,
        post_backup_command_error: RefCell<Option<crate::ui::error::Error>>,

        script_running: Cell<bool>,
        script_communication:
            RefCell<Option<crate::borg::Communication<crate::borg::task::UserScript>>>,

        #[template_child]
        command_line_args_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pre_backup_command_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pre_backup_command_test_button: TemplateChild<gtk::Button>,
        #[template_child]
        post_backup_command_test_button: TemplateChild<gtk::Button>,
        #[template_child]
        post_backup_command_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        shell_commands_detail: TemplateChild<gtk::Label>,

        #[property(get = Self::command_line_args, set = Self::set_command_line_args, type = String)]
        command_line_args: RefCell<Option<Vec<String>>>,
        #[property(get, set = Self::set_pre_backup_command)]
        pre_backup_command: RefCell<String>,
        #[property(get, set = Self::set_post_backup_command)]
        post_backup_command: RefCell<String>,

        // Tweaks
        #[property(get, set)]
        schedule_run_on_battery: Cell<bool>,

        // Change password page
        #[template_child]
        page_change_encryption_password: TemplateChild<adw::NavigationPage>,
        #[template_child]
        change_password_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        change_password_page_enter_password: TemplateChild<adw::ToolbarView>,
        #[template_child]
        encryption_preferences_group: TemplateChild<EncryptionPreferencesGroup>,
        #[template_child]
        change_password_page_spinner: TemplateChild<adw::ToolbarView>,
        #[template_child]
        change_password_button: TemplateChild<gtk::Button>,
        #[template_child]
        changing_password_spinner: TemplateChild<gtk::Spinner>,
        change_password_communication:
            RefCell<Option<crate::borg::Communication<crate::borg::task::KeyChangePassphrase>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PreferencesDialog {
        const NAME: &'static str = "PkPreferencesDialog";
        type Type = super::PreferencesDialog;
        type ParentType = adw::PreferencesWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PreferencesDialog {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn constructed(&self) {
            self.parent_constructed();
            self.load_config();
            self.obj().set_transient_for(Some(&main_ui().window()));
            self.shell_commands_detail
                .set_label(&crate::borg::scripts::ShellVariable::explanation_string_markup());
        }
    }

    impl WidgetImpl for PreferencesDialog {}

    impl WindowImpl for PreferencesDialog {
        fn close_request(&self) -> glib::Propagation {
            if self.close_real.get() {
                self.parent_close_request()
            } else {
                let obj = self.obj().clone();

                Handler::run(async move {
                    let imp = obj.imp();
                    imp.save_config().await?;
                    main_ui().page_detail().backup_page().refresh()?;

                    if imp.command_line_args_error.borrow().is_some() {
                        glib::MainContext::default().spawn_local(async move {
                            if let Some(err) = obj.imp().command_line_args_error.take() {
                                err.show().await;
                                obj.imp().command_line_args_error.replace(Some(err));
                            }
                        });
                    } else if imp.pre_backup_command_error.borrow().is_some() {
                        glib::MainContext::default().spawn_local(async move {
                            if let Some(err) = obj.imp().pre_backup_command_error.take() {
                                err.show().await;
                                obj.imp().pre_backup_command_error.replace(Some(err));
                            }
                        });
                    } else if imp.post_backup_command_error.borrow().is_some() {
                        glib::MainContext::default().spawn_local(async move {
                            if let Some(err) = obj.imp().post_backup_command_error.take() {
                                err.show().await;
                                obj.imp().post_backup_command_error.replace(Some(err));
                            }
                        });
                    } else {
                        imp.close_real.set(true);
                        obj.close();
                    }

                    Ok(())
                });

                glib::Propagation::Stop
            }
        }
    }

    impl AdwWindowImpl for PreferencesDialog {}

    impl PreferencesWindowImpl for PreferencesDialog {}

    #[gtk::template_callbacks]
    impl PreferencesDialog {
        fn config(&self) -> Result<crate::config::Backup> {
            match BACKUP_CONFIG.load().try_get(self.config_id.get().unwrap()) {
                Ok(backup) => Ok(backup.clone()),
                Err(err) => Err(crate::ui::Error::from(err)),
            }
        }

        pub async fn save_config(&self) -> Result<()> {
            BACKUP_CONFIG
                .try_update(|c| {
                    let backup = c.try_get_mut(self.config_id.get().unwrap())?;
                    backup.title = self.config_title.borrow().trim().to_string();

                    if !self.pre_backup_command.borrow().is_empty() {
                        backup.user_scripts.insert(
                            UserScriptKind::PreBackup,
                            self.pre_backup_command.borrow().clone(),
                        );
                    } else {
                        backup.user_scripts.remove(&UserScriptKind::PreBackup);
                    }

                    if !self.post_backup_command.borrow().is_empty() {
                        backup.user_scripts.insert(
                            UserScriptKind::PostBackup,
                            self.post_backup_command.borrow().clone(),
                        );
                    } else {
                        backup.user_scripts.remove(&UserScriptKind::PostBackup);
                    }

                    backup.repo.set_settings(Some(BackupSettings {
                        command_line_args: self.command_line_args.borrow().clone(),
                    }));

                    backup.schedule.settings.run_on_battery = self.schedule_run_on_battery.get();

                    Ok(())
                })
                .await
        }

        pub fn load_config(&self) {
            match self.config() {
                Ok(backup) => {
                    self.obj().set_config_title(backup.title());
                    self.title_pref_group.set_description(Some(&gettextf("The title of this backup configuration. Will be displayed as “{}” when left empty.", &[&backup.repo.title_fallback()])));

                    self.obj().set_pre_backup_command(
                        backup
                            .user_scripts
                            .get(&UserScriptKind::PreBackup)
                            .cloned()
                            .unwrap_or_default(),
                    );
                    self.obj().set_post_backup_command(
                        backup
                            .user_scripts
                            .get(&UserScriptKind::PostBackup)
                            .cloned()
                            .unwrap_or_default(),
                    );

                    if let Some(settings) = backup.repo.settings() {
                        self.obj().set_command_line_args(
                            settings
                                .command_line_args
                                .map(|a| a.join(" "))
                                .unwrap_or("".to_string()),
                        );
                    }

                    self.obj()
                        .set_schedule_run_on_battery(backup.schedule.settings.run_on_battery);
                }
                Err(err) => {
                    glib::MainContext::default().spawn_local(async move {
                        err.show().await;
                    });
                    self.obj().close();
                }
            };
        }

        fn command_line_args(&self) -> String {
            self.command_line_args
                .borrow()
                .clone()
                .map(|v| v.join(" "))
                .unwrap_or("".to_string())
        }

        fn set_command_line_args(&self, args: String) {
            match crate::ui::utils::borg::parse_borg_command_line_args(&args) {
                Ok(args) => {
                    self.command_line_args_entry.remove_css_class("error");
                    self.command_line_args.set(Some(args));
                    self.command_line_args_error.replace(None);
                }
                Err(err) => {
                    self.command_line_args.set(Some(vec![]));
                    self.command_line_args_entry.add_css_class("error");
                    self.command_line_args_error.replace(Some(err));
                }
            }
        }

        fn validate_shell_command(command: &str) -> Result<&str> {
            if shell_words::split(command).is_ok() {
                Ok(command)
            } else {
                Err(Message::new(
                    gettext("Shell command invalid"),
                    gettext("Please check for missing closing quotes."),
                )
                .into())
            }
        }

        fn set_pre_backup_command(&self, command: String) {
            match Self::validate_shell_command(&command) {
                Ok(_) => {
                    self.pre_backup_command_entry.remove_css_class("error");
                    self.pre_backup_command.set(command);
                    self.pre_backup_command_error.replace(None);
                }
                Err(err) => {
                    self.pre_backup_command.set(String::new());
                    self.pre_backup_command_entry.add_css_class("error");
                    self.pre_backup_command_error.replace(Some(err));
                }
            }
        }

        fn set_post_backup_command(&self, command: String) {
            match Self::validate_shell_command(&command) {
                Ok(_) => {
                    self.post_backup_command_entry.remove_css_class("error");
                    self.post_backup_command.set(command);
                    self.post_backup_command_error.replace(None);
                }
                Err(err) => {
                    self.post_backup_command.set(String::new());
                    self.post_backup_command_entry.add_css_class("error");
                    self.post_backup_command_error.replace(Some(err));
                }
            }
        }

        async fn test_run_script(
            &self,
            kind: UserScriptKind,
            config: crate::config::Backup,
            run_info: Option<crate::config::history::RunInfo>,
        ) {
            self.script_running.set(true);

            match &kind {
                UserScriptKind::PreBackup => {
                    self.pre_backup_command_test_button
                        .set_icon_name("stop-large-symbolic");
                    self.post_backup_command_test_button.set_sensitive(false);
                }
                UserScriptKind::PostBackup => {
                    self.pre_backup_command_test_button.set_sensitive(false);
                    self.post_backup_command_test_button
                        .set_icon_name("stop-large-symbolic");
                }
            }

            let mut command =
                crate::borg::Command::<crate::borg::task::UserScript>::new(config.clone());
            self.script_communication
                .replace(Some(command.communication.clone()));
            command.task.set_kind(kind.clone());
            command.task.set_run_info(run_info.clone());
            if let Err(err) = crate::ui::utils::borg::exec(command, &QuitGuard::default())
                .await
                .into_message(gettext("Error Running Shell Command"))
            {
                err.show_transient_for(&*self.obj()).await;
            }

            match &kind {
                UserScriptKind::PreBackup => {
                    self.pre_backup_command_test_button
                        .set_icon_name("play-large-symbolic");
                    self.post_backup_command_test_button.set_sensitive(true);
                }
                UserScriptKind::PostBackup => {
                    self.pre_backup_command_test_button.set_sensitive(true);
                    self.post_backup_command_test_button
                        .set_icon_name("play-large-symbolic");
                }
            }

            self.script_communication.take();
            self.script_running.set(false);
        }

        async fn abort_test_run_script(&self) {
            if let Some(communication) = self.script_communication.take() {
                debug!("Aborting script test");

                communication
                    .set_instruction(crate::borg::Instruction::Abort(crate::borg::Abort::User));
            }
        }

        #[template_callback]
        async fn test_pre_backup_command(&self) {
            if self.script_running.get() {
                self.abort_test_run_script().await;
                return;
            }

            let command = self.obj().pre_backup_command();

            if !command.is_empty() {
                if let Ok(mut config) = self.config() {
                    config
                        .user_scripts
                        .insert(UserScriptKind::PreBackup, command);

                    self.test_run_script(UserScriptKind::PreBackup, config, None)
                        .await;
                }
            }
        }

        #[template_callback]
        async fn test_post_backup_command(&self) {
            if self.script_running.get() {
                self.abort_test_run_script().await;
                return;
            }

            let command = self.obj().post_backup_command();

            if !command.is_empty() {
                if let Ok(mut config) = self.config() {
                    // Check if there is already a last RunInfo, if so, use that one
                    let run_info = if let Some(run_info) = BACKUP_HISTORY
                        .load()
                        .try_get(self.config_id.get().unwrap())
                        .ok()
                        .and_then(|history| history.last_completed())
                    {
                        run_info.clone()
                    } else {
                        // Create one from scratch with random values
                        crate::config::history::RunInfo::new(
                            &config,
                            crate::borg::Outcome::Completed {
                                stats: crate::borg::Stats::new_example(),
                            },
                            Default::default(),
                        )
                    };

                    config
                        .user_scripts
                        .insert(UserScriptKind::PostBackup, command);

                    self.test_run_script(UserScriptKind::PostBackup, config, Some(run_info))
                        .await;
                }
            }
        }

        #[template_callback]
        async fn change_password(&self) {
            let encrypted = self.config().map(|cfg| cfg.encrypted).unwrap_or_default();
            self.encryption_preferences_group.reset(encrypted);

            self.obj()
                .push_subpage(&*self.page_change_encryption_password);
            self.obj()
                .set_default_widget(Some(&*self.change_password_button));
        }

        async fn do_change_password_confirm(&self) -> Result<()> {
            self.page_change_encryption_password.set_can_pop(false);
            self.change_password_stack
                .set_visible_child(&*self.change_password_page_spinner);
            self.changing_password_spinner.set_spinning(true);

            let encrypted = self.encryption_preferences_group.encrypted();
            let password = self.encryption_preferences_group.validated_password()?;

            let config = self.config()?;

            let mut command: borg::Command<borg::task::KeyChangePassphrase> =
                borg::Command::new(config.clone());
            command.task.set_new_password(password.clone());
            self.change_password_communication
                .replace(Some(command.communication.clone()));
            crate::ui::utils::borg::exec(command, &QuitGuard::default())
                .await
                .into_message(gettext("Change Encryption Password Error"))?;
            self.change_password_communication.take();

            self.obj().pop_subpage();
            self.change_password_dismissed();
            self.obj().add_toast(
                adw::Toast::builder()
                    .title(gettext("Password changed successfully"))
                    .build(),
            );

            if config.encrypted != encrypted {
                BACKUP_CONFIG
                    .try_update(|config| {
                        config
                            .try_get_mut(self.config_id.get().unwrap())
                            .map(|cfg| cfg.encrypted = encrypted)?;
                        Ok(())
                    })
                    .await?;

                if !encrypted {
                    crate::ui::utils::password_storage::remove_password(&config, true).await?;
                }
            }

            // Save to keyring
            if let Some(password) = password {
                crate::ui::utils::password_storage::store_password(&config, &password).await?;
            }

            Ok(())
        }

        #[template_callback]
        async fn change_password_confirm(&self) {
            if let Err(err) = self.do_change_password_confirm().await {
                Handler::new()
                    .error_transient_for(self.obj().clone())
                    .spawn(async { Err(err) });
                self.change_password_cancel();
            }
        }

        #[template_callback]
        fn change_password_cancel(&self) {
            if let Some(communication) = self.change_password_communication.take() {
                debug!("Aborting change password");

                communication
                    .set_instruction(crate::borg::Instruction::Abort(crate::borg::Abort::User));
            }

            self.page_change_encryption_password.set_can_pop(true);
            self.change_password_stack
                .set_visible_child(&*self.change_password_page_enter_password);
            self.changing_password_spinner.set_spinning(false);
        }

        #[template_callback]
        fn change_password_dismissed(&self) {
            self.page_change_encryption_password.set_can_pop(true);
            self.change_password_stack
                .set_visible_child(&*self.change_password_page_enter_password);
            self.obj().set_default_widget(gtk::Widget::NONE);
        }
    }
}

glib::wrapper! {
    pub struct PreferencesDialog(ObjectSubclass<imp::PreferencesDialog>)
        @extends gtk::Widget, gtk::Window, adw::Window, adw::PreferencesWindow;
}

impl PreferencesDialog {
    pub fn new(config_id: ConfigId) -> Self {
        glib::Object::builder()
            .property("config-id", config_id)
            .build()
    }
}
