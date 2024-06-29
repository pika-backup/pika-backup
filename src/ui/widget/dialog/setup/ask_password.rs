use crate::config;
use crate::ui::prelude::*;
use adw::prelude::*;
use adw::subclass::prelude::*;

mod imp {
    use glib::subclass::Signal;

    use crate::ui::widget::PkDialogPageImpl;

    use super::*;
    use std::sync::OnceLock;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(file = "ask_password.ui")]
    pub struct SetupAskPasswordPage {
        #[template_child]
        pub(super) password_entry: TemplateChild<adw::PasswordEntryRow>,
        #[template_child]
        pub(super) continue_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SetupAskPasswordPage {
        const NAME: &'static str = "PkSetupAskPasswordPage";
        type Type = super::SetupAskPasswordPage;
        type ParentType = DialogPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SetupAskPasswordPage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![Signal::builder("continue")
                    .param_types([config::Password::static_type()])
                    .build()]
            })
        }
    }
    impl WidgetImpl for SetupAskPasswordPage {}
    impl NavigationPageImpl for SetupAskPasswordPage {
        fn showing(&self) {
            self.password_entry.set_text("");
        }

        fn shown(&self) {
            self.password_entry.grab_focus();
        }
    }
    impl PkDialogPageImpl for SetupAskPasswordPage {}

    #[gtk::template_callbacks]
    impl SetupAskPasswordPage {
        fn emit_continue(&self, password: config::Password) {
            self.obj().emit_by_name::<()>("continue", &[&password]);
        }

        #[template_callback]
        async fn on_continue_button(&self) {
            let password = self.password_entry.text();
            if password.is_empty() {
                return;
            }

            self.emit_continue(config::Password::new(password.to_string()))
        }
    }
}

glib::wrapper! {
    pub struct SetupAskPasswordPage(ObjectSubclass<imp::SetupAskPasswordPage>)
    @extends DialogPage, adw::NavigationPage, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SetupAskPasswordPage {}
