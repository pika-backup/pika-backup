use crate::config;
use crate::ui::prelude::*;
use adw::prelude::*;
use adw::subclass::prelude::*;

mod imp {
    use super::*;
    use crate::config;
    use std::{cell::Cell, marker::PhantomData};

    #[derive(Debug, Default, glib::Properties, gtk::CompositeTemplate)]
    #[template(file = "encryption_settings.ui")]
    #[properties(wrapper_type = super::EncryptionSettings)]
    pub struct EncryptionSettings {
        #[template_child]
        password_entry: TemplateChild<adw::PasswordEntryRow>,
        #[template_child]
        password_confirm_entry: TemplateChild<adw::PasswordEntryRow>,
        #[template_child]
        password_quality_bar: TemplateChild<gtk::LevelBar>,
        #[template_child]
        revealer: TemplateChild<gtk::Revealer>,

        #[property(get, set)]
        encrypted: Cell<bool>,
        #[property(get = Self::description)]
        description: PhantomData<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EncryptionSettings {
        const NAME: &'static str = "PkEncryptionSettings";
        type Type = super::EncryptionSettings;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EncryptionSettings {
        fn constructed(&self) {
            self.parent_constructed();

            self.password_quality_bar
                .add_offset_value(gtk::LEVEL_BAR_OFFSET_LOW, 7.0);
            self.password_quality_bar
                .add_offset_value(gtk::LEVEL_BAR_OFFSET_HIGH, 5.0);
            self.password_quality_bar
                .add_offset_value(gtk::LEVEL_BAR_OFFSET_FULL, 3.0);
        }

        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }
        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }
        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }
    }
    impl WidgetImpl for EncryptionSettings {}
    impl BoxImpl for EncryptionSettings {}

    #[gtk::template_callbacks]
    impl EncryptionSettings {
        #[template_callback]
        pub fn on_switch_active(&self) {
            if !self.encrypted.get() {
                self.reset();
            }
        }

        pub(super) fn reset(&self) {
            self.password_entry.set_text("");
            self.password_confirm_entry.set_text("");
        }

        fn description(&self) -> String {
            gettext("The data stored in encrypted backups is password protected. If encryption is used, the password is required for accessing your backups.")
        }

        pub fn validated_password(&self) -> Result<Option<config::Password>> {
            if self.encrypted.get() {
                let password = self.password_entry.text().to_string();
                if password.is_empty() {
                    return Err(Message::new(
                        gettext("No Password Provided"),
                        gettext("To use encryption a password must be provided."),
                    )
                    .into());
                }

                if password != self.password_confirm_entry.text() {
                    return Err(Message::short(gettext("Entered passwords do not match.")).into());
                }

                Ok(Some(crate::config::Password::new(password)))
            } else {
                Ok(None)
            }
        }

        pub fn score_password(password: &str) -> f64 {
            if let Ok(pw_check) = zxcvbn::zxcvbn(password, &[]) {
                if pw_check.score() > 3 {
                    let n = pw_check.guesses_log10();
                    if (12.0..13.0).contains(&n) {
                        5.
                    } else if (13.0..14.0).contains(&n) {
                        6.
                    } else if n > 14.0 {
                        7.
                    } else {
                        4.
                    }
                } else {
                    pw_check.score() as f64
                }
            } else {
                0.
            }
        }

        #[template_callback]
        fn password_value_changed(&self) {
            let password = self.password_entry.text();
            self.password_quality_bar
                .set_value(Self::score_password(&password));

            // Show warning highlight if passwords don't match
            if !self.password_confirm_entry.text().is_empty() {
                if self.password_entry.text() == self.password_confirm_entry.text() {
                    self.password_confirm_entry.add_css_class("success");
                    self.password_confirm_entry.remove_css_class("warning");
                } else {
                    self.password_confirm_entry.remove_css_class("success");
                    self.password_confirm_entry.add_css_class("warning");
                }
            } else {
                self.password_confirm_entry.remove_css_class("success");
                self.password_confirm_entry.remove_css_class("warning");
            }
        }
    }
}

glib::wrapper! {
    pub struct EncryptionSettings(ObjectSubclass<imp::EncryptionSettings>)
        @extends gtk::Box, gtk::Widget;
}

impl EncryptionSettings {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn reset(&self, encrypted: bool) {
        self.set_encrypted(encrypted);
        self.imp().reset();
    }

    pub fn validated_password(&self) -> Result<Option<config::Password>> {
        self.imp().validated_password()
    }
}
