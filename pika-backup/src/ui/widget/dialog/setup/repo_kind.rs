use adw::prelude::*;
use adw::subclass::prelude::*;

use super::SetupAction;
use crate::ui::prelude::*;

mod imp {
    use std::cell::Cell;
    use std::sync::OnceLock;

    use glib::subclass::Signal;

    use super::*;
    use crate::ui::widget::PkDialogPageImpl;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[template(file = "repo_kind.ui")]
    #[properties(wrapper_type = super::SetupRepoKindPage)]
    pub struct SetupRepoKindPage {
        #[property(get, set)]
        prop: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SetupRepoKindPage {
        const NAME: &'static str = "PkSetupRepoKindPage";
        type Type = super::SetupRepoKindPage;
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
    impl ObjectImpl for SetupRepoKindPage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("continue")
                        .param_types([SetupAction::static_type()])
                        .build(),
                ]
            })
        }
    }
    impl WidgetImpl for SetupRepoKindPage {}
    impl NavigationPageImpl for SetupRepoKindPage {}
    impl PkDialogPageImpl for SetupRepoKindPage {}

    #[gtk::template_callbacks]
    impl SetupRepoKindPage {
        #[template_callback]
        fn on_create_new(&self) {
            self.obj()
                .emit_by_name::<()>("continue", &[&SetupAction::Init]);
        }

        #[template_callback]
        fn on_use_existing(&self) {
            self.obj()
                .emit_by_name::<()>("continue", &[&SetupAction::AddExisting]);
        }
    }
}

glib::wrapper! {
    pub struct SetupRepoKindPage(ObjectSubclass<imp::SetupRepoKindPage>)
    @extends DialogPage, adw::NavigationPage, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SetupRepoKindPage {}
