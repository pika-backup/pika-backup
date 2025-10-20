use adw::prelude::*;
use adw::subclass::prelude::*;
use common::borg;

use super::ArchiveParams;
use super::transfer_option::SetupTransferOption;
use crate::ui::prelude::*;

mod imp {

    use std::cell::Cell;
    use std::sync::OnceLock;

    use glib::subclass::Signal;
    use itertools::Itertools;

    use super::*;
    use crate::ui::widget::PkDialogPageImpl;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[template(file = "transfer_settings.ui")]
    #[properties(wrapper_type = super::SetupTransferSettingsPage)]
    pub struct SetupTransferSettingsPage {
        #[property(get)]
        has_suggestions: Cell<bool>,

        #[template_child]
        pub(super) transfer_suggestions: TemplateChild<adw::PreferencesGroup>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SetupTransferSettingsPage {
        const NAME: &'static str = "PkSetupTransferSettingsPage";
        type Type = super::SetupTransferSettingsPage;
        type ParentType = DialogPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for SetupTransferSettingsPage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("continue")
                        .param_types([Option::<ArchiveParams>::static_type()])
                        .build(),
                ]
            })
        }
    }
    impl WidgetImpl for SetupTransferSettingsPage {}
    impl NavigationPageImpl for SetupTransferSettingsPage {}
    impl PkDialogPageImpl for SetupTransferSettingsPage {}

    #[gtk::template_callbacks]
    impl SetupTransferSettingsPage {
        fn emit_continue(&self, prefix: Option<ArchiveParams>) {
            self.obj().emit_by_name::<()>("continue", &[&prefix]);
        }

        #[template_callback]
        fn on_skip_button(&self) {
            self.emit_continue(None);
        }

        pub(super) fn set_archives(&self, archives: Vec<borg::ListArchive>) {
            let archive_params: Vec<_> = archives
                .into_iter()
                .map(ArchiveParams::from)
                .rev()
                .collect();

            let valid_prefixes: Vec<_> = archive_params
                .iter()
                .map(|x| &x.prefix)
                .duplicates()
                .collect();

            let mut options = archive_params
                .iter()
                .filter(|x| valid_prefixes.contains(&&x.prefix))
                .unique_by(|x| (&x.prefix, &x.parsed, &x.hostname, &x.username))
                .peekable();

            self.has_suggestions.set(options.peek().is_some());
            self.obj().notify_has_suggestions();

            let mut child = self.transfer_suggestions.first_child();
            while let Some(row) = child {
                if let Some(row) = row.downcast_ref::<SetupTransferOption>() {
                    self.transfer_suggestions.remove(row);
                }

                child = row.next_sibling();
            }

            for suggestion in options.take(10) {
                let row = SetupTransferOption::new(suggestion);

                let obj = self.obj();
                row.transfer_row().connect_activated(clone!(
                    #[weak]
                    obj,
                    #[strong]
                    suggestion,
                    move |_| obj.imp().emit_continue(Some(suggestion.clone()))
                ));

                self.transfer_suggestions.add(&row);
            }
        }
    }
}

glib::wrapper! {
    pub struct SetupTransferSettingsPage(ObjectSubclass<imp::SetupTransferSettingsPage>)
    @extends DialogPage, adw::NavigationPage, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SetupTransferSettingsPage {
    pub(super) fn set_archives(&self, archives: Vec<borg::ListArchive>) {
        self.imp().set_archives(archives);
    }
}
