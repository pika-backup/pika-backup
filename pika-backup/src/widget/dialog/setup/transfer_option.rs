use adw::subclass::prelude::*;

use super::ArchiveParams;
use crate::prelude::*;

mod imp {

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(file = "transfer_option.ui")]
    pub struct SetupTransferOption {
        #[template_child]
        hostname: TemplateChild<gtk::Label>,
        #[template_child]
        username: TemplateChild<gtk::Label>,
        #[template_child]
        prefix: TemplateChild<gtk::Label>,
        #[template_child]
        include_box: TemplateChild<adw::WrapBox>,
        #[template_child]
        exclude_box: TemplateChild<adw::WrapBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SetupTransferOption {
        const NAME: &'static str = "PkSetupTransferOption";
        type Type = super::SetupTransferOption;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SetupTransferOption {}
    impl WidgetImpl for SetupTransferOption {}
    impl ListBoxRowImpl for SetupTransferOption {}
    impl PreferencesRowImpl for SetupTransferOption {}
    impl ActionRowImpl for SetupTransferOption {}

    #[gtk::template_callbacks]
    impl SetupTransferOption {
        pub(super) fn set_suggestion(&self, suggestion: &ArchiveParams) {
            self.hostname.set_label(&suggestion.hostname);
            self.username.set_label(&suggestion.username);
            self.prefix.set_label(
                &suggestion
                    .prefix
                    .as_ref()
                    .map(|x| x.to_string())
                    .unwrap_or_else(|| gettext("None")),
            );

            for include in suggestion.parsed.include.iter() {
                let tag = crate::widget::LocationTag::from_path(include.clone());
                self.include_box.append(&tag.build());
            }

            for exclude in suggestion.parsed.exclude.iter() {
                let tag = crate::widget::LocationTag::from_exclude(exclude.clone().into_relative());
                self.exclude_box.append(&tag.build());
            }
        }
    }
}

glib::wrapper! {
    pub struct SetupTransferOption(ObjectSubclass<imp::SetupTransferOption>)
    @extends adw::ActionRow, adw::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Actionable;
}

impl SetupTransferOption {
    pub fn new(suggestion: &ArchiveParams) -> Self {
        let dialog: Self = glib::Object::new();
        dialog.imp().set_suggestion(suggestion);
        dialog
    }
}
