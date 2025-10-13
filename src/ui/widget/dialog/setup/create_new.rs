use adw::subclass::prelude::*;

use crate::ui::prelude::*;

mod imp {
    use super::*;
    use crate::ui::widget::{PkSpinnerPageImpl, SpinnerPage};

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(file = "create_new.ui")]
    pub struct SetupCreateNewPage {}

    #[glib::object_subclass]
    impl ObjectSubclass for SetupCreateNewPage {
        const NAME: &'static str = "PkSetupCreateNewPage";
        type Type = super::SetupCreateNewPage;
        type ParentType = SpinnerPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SetupCreateNewPage {}
    impl WidgetImpl for SetupCreateNewPage {}
    impl NavigationPageImpl for SetupCreateNewPage {}
    impl PkSpinnerPageImpl for SetupCreateNewPage {}

    impl SetupCreateNewPage {}
}

glib::wrapper! {
    pub struct SetupCreateNewPage(ObjectSubclass<imp::SetupCreateNewPage>)
    @extends DialogPage, adw::NavigationPage, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SetupCreateNewPage {}
