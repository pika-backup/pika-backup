use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::WeakRef;

mod imp {
    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::DialogPage)]
    pub struct DialogPage {
        #[property(get, set, nullable)]
        default_widget: WeakRef<gtk::Widget>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DialogPage {
        const NAME: &'static str = "PkDialogPage";
        type Type = super::DialogPage;
        type ParentType = adw::NavigationPage;
    }

    #[glib::derived_properties]
    impl ObjectImpl for DialogPage {}
    impl WidgetImpl for DialogPage {}
    impl NavigationPageImpl for DialogPage {}
    impl PkDialogPageImpl for DialogPage {}

    #[gtk::template_callbacks]
    impl DialogPage {}
}

glib::wrapper! {
    pub struct DialogPage(ObjectSubclass<imp::DialogPage>)
    @extends adw::NavigationPage, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl DialogPage {}

pub trait PkDialogPageImpl: NavigationPageImpl + WidgetImpl + ObjectImpl {}
unsafe impl<T: PkDialogPageImpl> IsSubclassable<T> for DialogPage {}

#[allow(dead_code)]
pub trait DialogPageExt: IsA<DialogPage> {
    fn set_default_widget(&self, widget: Option<impl IsA<gtk::Widget>>) {
        <self::DialogPage>::set_default_widget(self.upcast_ref::<DialogPage>(), widget);
    }

    fn default_widget(&self) -> Option<gtk::Widget> {
        <self::DialogPage>::default_widget(self.upcast_ref::<DialogPage>())
    }
}

impl<T: IsA<DialogPage>> DialogPageExt for T {}
