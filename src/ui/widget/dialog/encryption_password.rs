use crate::config;
use crate::ui::prelude::*;

use adw::prelude::*;
use adw::subclass::prelude::*;

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(file = "encryption_password.ui")]
    pub struct EncryptionPasswordDialog {
        #[template_child]
        pub(super) password: TemplateChild<gtk::PasswordEntry>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EncryptionPasswordDialog {
        const NAME: &'static str = "PkEncryptionPasswordDialog";
        type Type = super::EncryptionPasswordDialog;
        type ParentType = adw::MessageDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EncryptionPasswordDialog {}
    impl WidgetImpl for EncryptionPasswordDialog {}
    impl WindowImpl for EncryptionPasswordDialog {}
    impl MessageDialogImpl for EncryptionPasswordDialog {}

    #[gtk::template_callbacks]
    impl EncryptionPasswordDialog {}
}

glib::wrapper! {
    pub struct EncryptionPasswordDialog(ObjectSubclass<imp::EncryptionPasswordDialog>)
    @extends adw::MessageDialog, gtk::Window, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl EncryptionPasswordDialog {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub async fn present_with(
        self,
        transient_for: &impl IsA<gtk::Window>,
        repo: &config::Repository,
        purpose: &str,
        keyring_error: Option<&str>,
    ) -> Option<config::Password> {
        self.set_transient_for(Some(transient_for));

        let mut body = gettextf(
            "The operation “{}” requires the encryption password of the repository on “{}”.",
            &[purpose, &repo.location()],
        );

        if let Some(keyring_error) = &keyring_error {
            body.push_str(&format!("\n\n{}", keyring_error));
        }

        self.set_body(&body);
        self.imp().password.grab_focus();
        let response = self.clone().choose_future().await;
        let password = config::Password::new(self.imp().password.text().to_string());

        if response == "apply" {
            Some(password)
        } else {
            None
        }
    }
}
