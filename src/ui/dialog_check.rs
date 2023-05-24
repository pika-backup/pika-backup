use adw::prelude::*;
use adw::subclass::prelude::*;

use crate::ui::prelude::*;

mod imp {
    use super::*;
    use glib::signal::Inhibit;
    use glib::Properties;
    use once_cell::unsync::OnceCell;
    use std::cell::{Cell, RefCell};

    #[derive(Debug, Default, Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::DialogCheck)]
    #[template(file = "dialog_check.ui")]
    pub struct DialogCheck {
        #[property(get, set, construct_only)]
        pub config_id: OnceCell<ConfigId>,

        #[property(get, set)]
        pub verify_data: Cell<bool>,

        #[property(get, set)]
        pub repair: Cell<bool>,

        #[template_child]
        button_run: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DialogCheck {
        const NAME: &'static str = "DialogCheck";
        type Type = super::DialogCheck;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DialogCheck {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn constructed(&self) {
            self.parent_constructed();
            self.obj().set_transient_for(Some(&main_ui().window()));

            let obj = self.obj();
            self.obj().connect_repair_notify(|dialog| {
                let imp = dialog.imp();
                if imp.repair.get() {
                    imp.button_run.add_css_class("destructive-action");
                    imp.button_run.remove_css_class("suggested-action");
                    imp.button_run.set_label(&gettext("Perform Repair"));
                } else {
                    imp.button_run.remove_css_class("destructive-action");
                    imp.button_run.add_css_class("suggested-action");
                    imp.button_run.set_label(&gettext("Perform Check"));
                }
            });
        }
    }

    impl WidgetImpl for DialogCheck {}

    impl WindowImpl for DialogCheck {
        fn close_request(&self) -> Inhibit {
            Inhibit(false)
        }
    }

    impl AdwWindowImpl for DialogCheck {}

    #[gtk::template_callbacks]
    impl DialogCheck {
        fn config(&self) -> Result<crate::config::Backup> {
            match BACKUP_CONFIG
                .load()
                .get_result(self.config_id.get().unwrap())
            {
                Ok(backup) => Ok(backup.clone()),
                Err(err) => Err(crate::ui::Error::from(err)),
            }
        }

        #[template_callback]
        fn run(&self) {
            let obj = self.obj();
            obj.close();

            Handler::run(glib::clone!(@strong obj => async move {
                let config = obj.imp().config()?;

                let mut command = crate::borg::Command::<crate::borg::task::Check>::new(config.clone());
                command.task.set_verify_data(obj.imp().verify_data.get());
                command.task.set_repair(obj.imp().repair.get());

                let quit_guard = QuitGuard::default();
                crate::ui::utils::borg::exec(command, &quit_guard).await.into_message(gettext("Verify Archives Integrity"))?;
                Ok(())
            }));
        }
    }
}

glib::wrapper! {
    pub struct DialogCheck(ObjectSubclass<imp::DialogCheck>)
        @extends gtk::Widget, gtk::Window, adw::Window;
}

impl DialogCheck {
    pub fn new(config_id: ConfigId) -> Self {
        glib::Object::builder()
            .property("config-id", config_id)
            .build()
    }
}
