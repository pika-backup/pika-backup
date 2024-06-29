use crate::ui::prelude::*;
use adw::prelude::*;
use adw::subclass::prelude::*;

use super::SetupCommandLineArgs;

mod imp {
    use crate::ui::widget::{setup::SetupCommandLineArgs, PkDialogPageImpl};

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(file = "advanced_options.ui")]
    pub struct SetupAdvancedOptionsPage {
        #[template_child]
        pub(super) command_line_args_entry: TemplateChild<adw::EntryRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SetupAdvancedOptionsPage {
        const NAME: &'static str = "PkSetupAdvancedOptionsPage";
        type Type = super::SetupAdvancedOptionsPage;
        type ParentType = DialogPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SetupAdvancedOptionsPage {}
    impl WidgetImpl for SetupAdvancedOptionsPage {}
    impl NavigationPageImpl for SetupAdvancedOptionsPage {}
    impl PkDialogPageImpl for SetupAdvancedOptionsPage {}

    impl SetupAdvancedOptionsPage {
        pub(super) fn selected_command_line_args(&self) -> Result<SetupCommandLineArgs> {
            let command_line = self.command_line_args_entry.text();
            command_line.parse()
        }
    }
}

glib::wrapper! {
    pub struct SetupAdvancedOptionsPage(ObjectSubclass<imp::SetupAdvancedOptionsPage>)
    @extends DialogPage, adw::NavigationPage, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SetupAdvancedOptionsPage {
    pub fn selected_command_line_args(&self) -> Result<SetupCommandLineArgs> {
        self.imp().selected_command_line_args()
    }
}
