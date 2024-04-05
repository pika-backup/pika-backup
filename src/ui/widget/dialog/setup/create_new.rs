use crate::ui::prelude::*;
use adw::prelude::*;
use adw::subclass::prelude::*;

mod imp {
    use crate::ui::widget::dialog_page::PkDialogPageImpl;

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(file = "create_new.ui")]
    pub struct SetupCreateNewPage {
        #[template_child]
        creating_repository_spinner: TemplateChild<gtk::Spinner>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SetupCreateNewPage {
        const NAME: &'static str = "PkSetupCreateNewPage";
        type Type = super::SetupCreateNewPage;
        type ParentType = DialogPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SetupCreateNewPage {
        fn constructed(&self) {
            self.parent_constructed();
            self.creating_repository_spinner.connect_map(|s| s.start());
            self.creating_repository_spinner.connect_unmap(|s| s.stop());
        }
    }
    impl WidgetImpl for SetupCreateNewPage {}
    impl NavigationPageImpl for SetupCreateNewPage {}
    impl PkDialogPageImpl for SetupCreateNewPage {}

    #[gtk::template_callbacks]
    impl SetupCreateNewPage {}
}

glib::wrapper! {
    pub struct SetupCreateNewPage(ObjectSubclass<imp::SetupCreateNewPage>)
    @extends DialogPage, adw::NavigationPage, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SetupCreateNewPage {}
