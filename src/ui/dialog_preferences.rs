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

        command_line_args_error: RefCell<Option<crate::ui::error::Error>>,

        #[template_child]
        command_line_args_entry: TemplateChild<adw::EntryRow>,

        #[property(get = Self::command_line_args, set = Self::set_command_line_args, type = String)]
        command_line_args: RefCell<Option<Vec<String>>>,
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
            self.obj().set_transient_for(Some(&main_ui().window()));
        }
    }

    impl WidgetImpl for DialogPreferences {}

    impl WindowImpl for DialogPreferences {
        fn close_request(&self) -> Inhibit {
            BACKUP_CONFIG.update(|c| match c.get_result_mut(self.config_id.get().unwrap()) {
                Ok(backup) => {
                    backup.repo.set_settings(Some(BackupSettings {
                        command_line_args: self.command_line_args.borrow().clone(),
                    }));
                }
                Err(err) => {
                    glib::MainContext::default().spawn_local(async move {
                        crate::ui::Error::from(err).show().await;
                    });
                    self.obj().close();
                }
            });

            let _ = crate::ui::write_config();

            if self.command_line_args_error.borrow().is_some() {
                let obj = self.obj().clone();
                glib::MainContext::default().spawn_local(async move {
                    if let Some(err) = obj.imp().command_line_args_error.take() {
                        err.show().await;
                        obj.imp().command_line_args_error.replace(Some(err));
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
                    self.obj().set_command_line_args(
                        backup
                            .repo
                            .settings()
                            .and_then(|s| s.command_line_args)
                            .map(|a| a.join(" "))
                            .unwrap_or("".to_string()),
                    );
                }
                Err(err) => {
                    glib::MainContext::default().spawn_local(async move {
                        crate::ui::Error::from(err).show().await;
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
