use adw::prelude::*;
use adw::subclass::prelude::*;

use crate::prelude::*;

mod imp {
    use std::cell::RefCell;

    use super::*;
    use crate::widget::PkDialogPageImpl;
    use crate::widget::setup::SetupCommandLineArgs;

    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[template(file = "advanced_options.ui")]
    #[properties(wrapper_type = super::SetupAdvancedOptionsPage)]
    pub struct SetupAdvancedOptionsPage {
        #[template_child]
        pub(super) command_line_args_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        validation_label: TemplateChild<gtk::Label>,
        #[property(get)]
        command_line_args: RefCell<SetupCommandLineArgs>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SetupAdvancedOptionsPage {
        const NAME: &'static str = "PkSetupAdvancedOptionsPage";
        type Type = super::SetupAdvancedOptionsPage;
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
    impl ObjectImpl for SetupAdvancedOptionsPage {
        fn constructed(&self) {
            self.parent_constructed();
            if let Some(text) = self.command_line_args_entry.delegate() {
                text.update_relation(&[gtk::accessible::Relation::ErrorMessage(&[self
                    .validation_label
                    .upcast_ref()])]);
            }
        }
    }
    impl WidgetImpl for SetupAdvancedOptionsPage {}
    impl NavigationPageImpl for SetupAdvancedOptionsPage {
        fn hiding(&self) {
            if self.selected_command_line_args().is_err() {
                self.command_line_args_entry.set_text("");
            }
        }
    }
    impl PkDialogPageImpl for SetupAdvancedOptionsPage {}

    #[gtk::template_callbacks]
    impl SetupAdvancedOptionsPage {
        fn selected_command_line_args(&self) -> Result<SetupCommandLineArgs> {
            let command_line = self.command_line_args_entry.text();
            command_line.parse()
        }

        #[template_callback]
        fn on_command_line_args_changed(&self) {
            let res = self.selected_command_line_args();

            let args = match res {
                Ok(args) => {
                    self.command_line_args_entry.remove_css_class("error");
                    self.command_line_args_entry.delegate().inspect(|d| {
                        d.update_state(&[gtk::accessible::State::Invalid(
                            gtk::AccessibleInvalidState::False,
                        )]);
                    });
                    self.validation_label.set_label("");
                    args
                }
                Err(err) => {
                    self.command_line_args_entry.add_css_class("error");
                    self.validation_label
                        .set_label(err.message_secondary_text().unwrap_or_default());
                    self.command_line_args_entry.delegate().inspect(|d| {
                        d.update_state(&[gtk::accessible::State::Invalid(
                            gtk::AccessibleInvalidState::True,
                        )]);
                    });
                    SetupCommandLineArgs::NONE
                }
            };

            if args != self.command_line_args.replace(args.clone()) {
                self.obj().notify_command_line_args();
            }
        }
    }
}

glib::wrapper! {
    pub struct SetupAdvancedOptionsPage(ObjectSubclass<imp::SetupAdvancedOptionsPage>)
    @extends DialogPage, adw::NavigationPage, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SetupAdvancedOptionsPage {}
