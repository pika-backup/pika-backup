use adw::prelude::*;
use adw::subclass::prelude::*;

use crate::config::BackupSettings;
use crate::ui::prelude::*;

mod imp {
    use super::*;
    use glib::signal::Inhibit;
    use glib::Properties;
    use once_cell::unsync::OnceCell;
    use std::cell::RefCell;

    #[derive(Debug, Default, Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::DialogPreferences)]
    #[template(file = "dialog_preferences.ui")]
    pub struct DialogPreferences {
        #[property(get, set, construct_only)]
        pub config_id: OnceCell<ConfigId>,

        #[template_child]
        title_pref_group: TemplateChild<adw::PreferencesGroup>,

        #[template_child]
        title_entry: TemplateChild<adw::EntryRow>,

        #[property(get, set)]
        config_title: RefCell<String>,

        command_line_args_error: RefCell<Option<crate::ui::error::Error>>,
        pre_backup_command_error: RefCell<Option<crate::ui::error::Error>>,
        post_backup_command_error: RefCell<Option<crate::ui::error::Error>>,

        #[template_child]
        command_line_args_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pre_backup_command_entry: TemplateChild<adw::EntryRow>,
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
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DialogPreferences {
        const NAME: &'static str = "DialogPreferences";
        type Type = super::DialogPreferences;
        type ParentType = adw::PreferencesWindow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DialogPreferences {
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
                .set_label(&crate::ui::utils::scripts::ShellVariables::explanation_string_markup());
        }
    }

    impl WidgetImpl for DialogPreferences {}

    impl WindowImpl for DialogPreferences {
        fn close_request(&self) -> Inhibit {
            BACKUP_CONFIG.update(|c| match c.get_result_mut(self.config_id.get().unwrap()) {
                Ok(backup) => {
                    backup.title = self.config_title.borrow().trim().to_string();
                    backup.repo.set_settings(Some(BackupSettings {
                        command_line_args: self.command_line_args.borrow().clone(),
                        pre_backup_command: Some(self.pre_backup_command.borrow().clone())
                            .filter(|s| !s.is_empty()),
                        post_backup_command: Some(self.post_backup_command.borrow().clone())
                            .filter(|s| !s.is_empty()),
                    }));
                }
                Err(err) => {
                    glib::MainContext::default().spawn_local(async move {
                        crate::ui::Error::from(err).show().await;
                    });
                    self.obj().close();
                }
            });

            Handler::handle((|| {
                crate::ui::write_config()?;
                crate::ui::page_backup::refresh()?;
                Ok(())
            })());

            let obj = self.obj().clone();

            if self.command_line_args_error.borrow().is_some() {
                glib::MainContext::default().spawn_local(async move {
                    if let Some(err) = obj.imp().command_line_args_error.take() {
                        err.show().await;
                        obj.imp().command_line_args_error.replace(Some(err));
                    }
                });

                Inhibit(true)
            } else if self.pre_backup_command_error.borrow().is_some() {
                glib::MainContext::default().spawn_local(async move {
                    if let Some(err) = obj.imp().pre_backup_command_error.take() {
                        err.show().await;
                        obj.imp().pre_backup_command_error.replace(Some(err));
                    }
                });

                Inhibit(true)
            } else if self.post_backup_command_error.borrow().is_some() {
                glib::MainContext::default().spawn_local(async move {
                    if let Some(err) = obj.imp().post_backup_command_error.take() {
                        err.show().await;
                        obj.imp().post_backup_command_error.replace(Some(err));
                    }
                });

                Inhibit(true)
            } else {
                Inhibit(false)
            }
        }
    }

    impl AdwWindowImpl for DialogPreferences {}

    impl PreferencesWindowImpl for DialogPreferences {}

    impl DialogPreferences {
        pub fn load_config(&self) {
            match BACKUP_CONFIG.load().get_result(
                self.config_id
                    .get()
                    .expect("constructor should set config_id"),
            ) {
                Ok(backup) => {
                    self.obj().set_config_title(backup.title());
                    self.title_pref_group.set_description(Some(&gettextf("The title of this backup configuration. Will be displayed as “{}” when left empty.", &[&backup.repo.title_fallback()])));

                    if let Some(settings) = backup.repo.settings() {
                        self.obj().set_command_line_args(
                            settings
                                .command_line_args
                                .map(|a| a.join(" "))
                                .unwrap_or("".to_string()),
                        );
                        self.obj().set_pre_backup_command(
                            settings.pre_backup_command.unwrap_or("".to_string()),
                        );
                        self.obj().set_post_backup_command(
                            settings.post_backup_command.unwrap_or("".to_string()),
                        );
                    }
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
    }
}

glib::wrapper! {
    pub struct DialogPreferences(ObjectSubclass<imp::DialogPreferences>)
        @extends gtk::Widget, gtk::Window, adw::Window, adw::PreferencesWindow;
}

impl DialogPreferences {
    pub fn new(config_id: ConfigId) -> Self {
        glib::Object::builder()
            .property("config-id", config_id)
            .build()
    }
}
