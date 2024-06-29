use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::WeakRef;

pub use imp::DialogPagePropertiesExt;

mod imp {
    use std::cell::{Cell, RefCell};

    use super::*;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::DialogPage, ext_trait)]
    #[template(file = "dialog_page.ui")]
    pub struct DialogPage {
        #[property(get, set, nullable)]
        default_widget: WeakRef<gtk::Widget>,
        #[property(get, set = Self::set_subtitle, nullable)]
        subtitle: RefCell<Option<String>>,

        #[property(get, set)]
        show_continue_button: Cell<bool>,
        #[template_child]
        toolbar_view: TemplateChild<adw::ToolbarView>,
        #[template_child]
        content_box: TemplateChild<gtk::Box>,
        #[template_child]
        subtitle_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DialogPage {
        const NAME: &'static str = "PkDialogPage";
        type Type = super::DialogPage;
        type ParentType = adw::NavigationPage;
        type Interfaces = (gtk::Buildable,);

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for DialogPage {}
    impl WidgetImpl for DialogPage {}
    impl NavigationPageImpl for DialogPage {}
    impl BuildableImpl for DialogPage {
        fn add_child(&self, builder: &gtk::Builder, child: &glib::Object, type_: Option<&str>) {
            if let Some(widget) = child.downcast_ref::<gtk::Widget>() {
                self.content_box.append(widget);
            } else {
                self.parent_add_child(builder, child, type_);
            }
        }
    }

    impl DialogPage {
        fn set_subtitle(&self, subtitle: Option<String>) {
            self.subtitle_label
                .set_visible(subtitle.as_ref().is_some_and(|s| !s.is_empty()));
            self.subtitle.replace(subtitle);
        }
    }
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
pub trait DialogPageExt: IsA<DialogPage> + imp::DialogPagePropertiesExt {}
impl<T: IsA<DialogPage>> DialogPageExt for T {}
