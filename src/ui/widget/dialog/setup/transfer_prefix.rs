use crate::config;
use crate::ui::prelude::*;
use adw::prelude::*;
use adw::subclass::prelude::*;

mod imp {

    use std::sync::OnceLock;

    use glib::subclass::Signal;

    use crate::{config, ui::widget::PkDialogPageImpl};

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(file = "transfer_prefix.ui")]
    pub struct SetupTransferPrefixPage {
        #[template_child]
        pub(super) prefix: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) prefix_submit: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SetupTransferPrefixPage {
        const NAME: &'static str = "PkSetupTransferPrefixPage";
        type Type = super::SetupTransferPrefixPage;
        type ParentType = DialogPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SetupTransferPrefixPage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![Signal::builder("continue")
                    .param_types([config::ArchivePrefix::static_type()])
                    .build()]
            })
        }
    }
    impl WidgetImpl for SetupTransferPrefixPage {}
    impl NavigationPageImpl for SetupTransferPrefixPage {
        fn showing(&self) {
            self.parent_showing();
            self.prefix.grab_focus();
        }
    }
    impl PkDialogPageImpl for SetupTransferPrefixPage {}

    #[gtk::template_callbacks]
    impl SetupTransferPrefixPage {
        fn emit_continue(&self, prefix: config::ArchivePrefix) {
            self.obj().emit_by_name::<()>("continue", &[&prefix]);
        }

        pub(super) fn set_prefix(&self, prefix: &crate::config::ArchivePrefix) {
            self.prefix.set_text(&prefix.0);
        }

        #[template_callback]
        async fn on_submit_button(&self) {
            let prefix = self.prefix.text();
            self.emit_continue(config::ArchivePrefix::new(&prefix));
        }
    }
}

glib::wrapper! {
    pub struct SetupTransferPrefixPage(ObjectSubclass<imp::SetupTransferPrefixPage>)
    @extends DialogPage, adw::NavigationPage, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SetupTransferPrefixPage {
    pub fn set_prefix(&self, prefix: &config::ArchivePrefix) {
        self.imp().set_prefix(prefix);
    }
}
