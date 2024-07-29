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
        validation_label: TemplateChild<gtk::Label>,
        #[template_child]
        password_quality_bar: TemplateChild<gtk::LevelBar>,
        #[template_child]
        revealer: TemplateChild<gtk::Revealer>,

        #[property(get, set)]
        encrypted: Cell<bool>,
        #[property(get = Self::description)]
        description: PhantomData<String>,
        #[property(get)]
        valid: Cell<bool>,
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

    #[glib::derived_properties]
    impl ObjectImpl for EncryptionSettings {
        fn constructed(&self) {
            self.parent_constructed();

            self.password_quality_bar
                .add_offset_value(gtk::LEVEL_BAR_OFFSET_LOW, 3.0);
            self.password_quality_bar
                .add_offset_value(gtk::LEVEL_BAR_OFFSET_HIGH, 5.0);
            self.password_quality_bar
                .add_offset_value(gtk::LEVEL_BAR_OFFSET_FULL, 7.0);

            self.score_password();
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

            self.update_valid();
        }

        fn update_valid(&self) {
            let validated_password = self.validated_password().is_ok();
            if self.valid.replace(validated_password) != validated_password {
                self.obj().notify_valid()
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
                    return Err(Message::new(
                        gettext("Invalid Passwords"),
                        gettext("The passwords do not match"),
                    )
                    .into());
                }

                Ok(Some(crate::config::Password::new(password)))
            } else {
                Ok(None)
            }
        }

        pub fn score_password(&self) {
            let password = self.password_entry.text();
            let password_confirm = self.password_confirm_entry.text();

            let entropy = zxcvbn::zxcvbn(&password, &[]);

            // Score:
            // - 0: no password
            // - 1-4 rather easy to crack, in the order of magnitude of seconds to years
            // - 5 centuries
            let (score, feedback) = entropy
                .map(|e| {
                    let guesses_log10 = e.guesses_log10();
                    let score = if guesses_log10 < 3. {
                        // less than a second
                        1
                    } else if guesses_log10 < 6. {
                        // seconds
                        2
                    } else if guesses_log10 < 8. {
                        // minutes
                        3
                    } else if guesses_log10 < 10. {
                        // hours
                        4
                    } else if guesses_log10 < 12. {
                        // days
                        5
                    } else if guesses_log10 < 14. {
                        // months / a few years
                        6
                    } else {
                        // centuries
                        7
                    };

                    debug!(
                        "score: {}, time to crack: {}",
                        score,
                        e.crack_times().offline_slow_hashing_1e4_per_second()
                    );

                    (score, e.feedback().to_owned())
                })
                .unwrap_or((0, None));

            let validation_str = if score == 0 {
                // Translators: Password feedback: Empty password. All strings labelled like this must fit in a single line at 360 width, to prevent the label from ellipsizing.
                gettext("Enter a password")
            } else if !password_confirm.is_empty() && password != password_confirm {
                // Translators: Password feedback: The second password is not the same as the first
                gettext("Passwords do not match")
            } else if score < 7 {
                let warning = feedback
                    .as_ref()
                    .and_then(|f| f.warning())
                    .map(|w| match w {
                        zxcvbn::feedback::Warning::AWordByItselfIsEasyToGuess => {
                            // Translators: Password feedback: A single word was found. Words are fine, but it needs more than one.
                            gettext("A single word is easy to guess")
                        }
                        zxcvbn::feedback::Warning::CommonNamesAndSurnamesAreEasyToGuess | zxcvbn::feedback::Warning::NamesAndSurnamesByThemselvesAreEasyToGuess => {
                            // Translators: Password feedback: Common name detected
                            gettext("Avoid names on their own")
                        }
                        zxcvbn::feedback::Warning::DatesAreOftenEasyToGuess | zxcvbn::feedback::Warning::RecentYearsAreEasyToGuess => {
                            // Translators: Password feedback: Date detected
                            gettext("Avoid dates on their own")
                        }
                        zxcvbn::feedback::Warning::SequencesLikeAbcAreEasyToGuess => {
                            // Translators: Password feedback: Easy to guess sequence detected
                            gettext("Avoid sequences like “abc” or “6543”")
                        }
                        zxcvbn::feedback::Warning::RepeatsLikeAaaAreEasyToGuess | zxcvbn::feedback::Warning::RepeatsLikeAbcAbcAreOnlySlightlyHarderToGuess => {
                            // Translators: Password feedback: Repeated keys or sequence detected
                            gettext("Avoid repetitions like “aaa” or “abcabc”")
                        },
                        zxcvbn::feedback::Warning::StraightRowsOfKeysAreEasyToGuess | zxcvbn::feedback::Warning::ShortKeyboardPatternsAreEasyToGuess => {
                            // Translators: Password feedback: This is for passwords built from rows of keys, or similar spatial patterns on the keyboard
                            gettext("Avoid keyboard patterns")
                        }
                        zxcvbn::feedback::Warning::ThisIsACommonPassword
                        | zxcvbn::feedback::Warning::ThisIsATop100Password
                        | zxcvbn::feedback::Warning::ThisIsATop10Password => {
                            // Translators: Password feedback: Password was found in a list of common passwords
                            gettext("This is a very commonly used password")
                        }
                        zxcvbn::feedback::Warning::ThisIsSimilarToACommonlyUsedPassword => {
                            // Translators: Password feedback: Password is very similar to a password from the list of common passwords
                            gettext("This is similar to a commonly used password")
                        }
                    });

                warning.unwrap_or(gettext("Add a few more words"))
            } else {
                // Translators: Password feedback: This password is looking pretty good
                gettext("This looks like a strong password")
            };

            self.validation_label.set_text(&validation_str);

            self.password_quality_bar.set_value(score as f64);

            // Show warning highlight if passwords don't match
            let validation_error = !self.password_confirm_entry.text().is_empty()
                && self.password_entry.text() != self.password_confirm_entry.text();

            if validation_error {
                self.password_confirm_entry.add_css_class("error");
            } else {
                self.password_confirm_entry.remove_css_class("error");
            }

            self.update_valid();
        }

        #[template_callback]
        fn password_value_changed(&self) {
            self.score_password();
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
