use crate::ui::prelude::*;
use adw::prelude::*;
use adw::subclass::prelude::*;

use crate::ui::widget::EncryptionSettings;

mod imp {
    use std::sync::OnceLock;

    use adw::subclass::navigation_page::NavigationPageImplExt;
    use glib::subclass::Signal;

    use crate::ui::{error::HandleError, widget::dialog_page::PkDialogPageImpl};

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(file = "encryption.ui")]
    pub struct SetupEncryptionPage {
        #[template_child]
        pub(super) encryption_settings: TemplateChild<EncryptionSettings>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SetupEncryptionPage {
        const NAME: &'static str = "PkSetupEncryptionPage";
        type Type = super::SetupEncryptionPage;
        type ParentType = DialogPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SetupEncryptionPage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![Signal::builder("continue")
                    .param_types([Option::<crate::config::Password>::static_type()])
                    .build()]
            })
        }
    }
    impl WidgetImpl for SetupEncryptionPage {}
    impl NavigationPageImpl for SetupEncryptionPage {
        fn shown(&self) {
            self.parent_shown();

            self.encryption_settings.grab_focus();
        }
    }
    impl PkDialogPageImpl for SetupEncryptionPage {}

    #[gtk::template_callbacks]
    impl SetupEncryptionPage {
        fn emit_continue(&self, password: Option<crate::config::Password>) {
            self.obj().emit_by_name::<()>("continue", &[&password]);
        }

        #[template_callback]
        async fn on_create_clicked(&self) {
            if let Some(configured_password) = self
                .encryption_settings
                .validated_password()
                .handle_transient_for(&*self.obj())
                .await
            {
                self.emit_continue(configured_password);
            }
        }
    }
}

glib::wrapper! {
    pub struct SetupEncryptionPage(ObjectSubclass<imp::SetupEncryptionPage>)
    @extends DialogPage, adw::NavigationPage, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SetupEncryptionPage {
    pub fn reset(&self) {
        self.imp().encryption_settings.reset(true);
    }
}
