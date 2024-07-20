use adw::prelude::*;
use adw::subclass::prelude::*;

pub use imp::SpinnerPagePropertiesExt;

mod imp {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::SpinnerPage, ext_trait)]
    #[template(file = "spinner_page.ui")]
    pub struct SpinnerPage {
        #[property(get, set, nullable)]
        description: RefCell<Option<String>>,

        #[template_child]
        status_page: TemplateChild<adw::StatusPage>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SpinnerPage {
        const NAME: &'static str = "PkSpinnerPage";
        type Type = super::SpinnerPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for SpinnerPage {}
    impl WidgetImpl for SpinnerPage {}
    impl NavigationPageImpl for SpinnerPage {
        fn shown(&self) {
            self.status_page.grab_focus();
        }
    }

    impl SpinnerPage {}
}

glib::wrapper! {
    pub struct SpinnerPage(ObjectSubclass<imp::SpinnerPage>)
    @extends adw::NavigationPage, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SpinnerPage {}

pub trait PkSpinnerPageImpl: NavigationPageImpl + WidgetImpl + ObjectImpl {}
unsafe impl<T: PkSpinnerPageImpl> IsSubclassable<T> for SpinnerPage {}
