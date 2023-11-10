use crate::ui::prelude::*;
use adw::subclass::prelude::*;

mod imp {
    use std::cell::RefCell;

    use crate::{
        config::history::CheckOutcome,
        ui::{backup_status, widget::StatusRow},
    };

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "dialog_check_result.ui")]
    pub struct DialogCheckResult {
        pub config_id: RefCell<Option<ConfigId>>,

        #[template_child]
        status_row: TemplateChild<StatusRow>,
        #[template_child]
        detail_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DialogCheckResult {
        const NAME: &'static str = "PikaDialogCheckResult";
        type Type = super::DialogCheckResult;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DialogCheckResult {}
    impl WidgetImpl for DialogCheckResult {}
    impl WindowImpl for DialogCheckResult {}
    impl AdwWindowImpl for DialogCheckResult {}

    #[gtk::template_callbacks]
    impl DialogCheckResult {
        pub fn reload(&self) {
            if let Some(id) = &*self.config_id.borrow() {
                self.status_row
                    .set_from_backup_status(&backup_status::Display::new_check_status_from_id(id));

                let last_check = BACKUP_HISTORY
                    .load()
                    .try_get(id)
                    .ok()
                    .cloned()
                    .and_then(|history| history.last_check);

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
    pub struct DialogCheckResult(ObjectSubclass<imp::DialogCheckResult>)
        @extends gtk::Widget, gtk::Window, adw::Window;
}

impl DialogCheckResult {
    pub fn set_config_id(&self, config_id: Option<ConfigId>) {
        self.imp().config_id.replace(config_id);
        self.reload();
    }

    pub fn reload(&self) {
        self.imp().reload();
    }
}
