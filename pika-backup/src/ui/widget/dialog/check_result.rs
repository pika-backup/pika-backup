use adw::subclass::prelude::*;

use crate::ui::prelude::*;

mod imp {
    use std::cell::RefCell;

    use common::config::history::CheckOutcome;

    use super::*;
    use crate::ui::backup_status;
    use crate::ui::widget::StatusRow;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "check_result.ui")]
    pub struct CheckResultDialog {
        pub config_id: RefCell<Option<ConfigId>>,

        #[template_child]
        status_row: TemplateChild<StatusRow>,
        #[template_child]
        detail_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CheckResultDialog {
        const NAME: &'static str = "PkCheckResultDialog";
        type Type = super::CheckResultDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for CheckResultDialog {}
    impl WidgetImpl for CheckResultDialog {}
    impl AdwDialogImpl for CheckResultDialog {}

    #[gtk::template_callbacks]
    impl CheckResultDialog {
        pub fn reload(&self) {
            if let Some(id) = &*self.config_id.borrow() {
                self.status_row
                    .set_from_backup_status(&backup_status::Display::new_check_status_from_id(id));

                let last_check = BACKUP_HISTORY
                    .load()
                    .try_get(id)
                    .ok()
                    .cloned()
                    .and_then(|history| history.last_check().cloned());

                if let Some(check_result) = last_check {
                    self.detail_label.set_label(&match check_result.outcome {
                        CheckOutcome::Repair(log) | CheckOutcome::Error(log) => {
                            log.filter_hidden().to_string()
                        }
                        _ => "".to_string(),
                    });
                } else {
                    self.detail_label.set_label("");
                }
            }
        }
    }
}

glib::wrapper! {
    pub struct CheckResultDialog(ObjectSubclass<imp::CheckResultDialog>)
        @extends gtk::Widget, adw::Dialog,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl CheckResultDialog {
    pub fn set_config_id(&self, config_id: Option<ConfigId>) {
        self.imp().config_id.replace(config_id);
        self.reload();
    }

    pub fn reload(&self) {
        self.imp().reload();
    }
}
