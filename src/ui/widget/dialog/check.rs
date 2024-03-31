use adw::prelude::*;
use adw::subclass::prelude::*;

use crate::ui::prelude::*;

mod imp {
    use crate::borg::log_json::LogEntry;
    use crate::config::history::CheckRunInfo;

    use super::*;
    use glib::Properties;
    use once_cell::unsync::OnceCell;
    use std::cell::Cell;

    #[derive(Debug, Default, Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::CheckDialog)]
    #[template(file = "check.ui")]
    pub struct CheckDialog {
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
    impl ObjectSubclass for CheckDialog {
        const NAME: &'static str = "PkDialogCheck";
        type Type = super::CheckDialog;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for CheckDialog {
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

    impl WidgetImpl for CheckDialog {}

    impl WindowImpl for CheckDialog {
        fn close_request(&self) -> glib::Propagation {
            glib::Propagation::Proceed
        }
    }

    impl AdwWindowImpl for CheckDialog {}

    #[gtk::template_callbacks]
    impl CheckDialog {
        fn config(&self) -> Result<crate::config::Backup> {
            match BACKUP_CONFIG.load().try_get(self.config_id.get().unwrap()) {
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

                scopeguard::defer_on_success!({
                    main_ui()
                        .page_detail()
                        .archives_page()
                        .refresh_status();
                });

                let mut command =
                    crate::borg::Command::<crate::borg::task::Check>::new(config.clone());
                command.task.set_verify_data(obj.imp().verify_data.get());
                let repair = obj.imp().repair.get();
                command.task.set_repair(repair);

                let quit_guard = QuitGuard::default();
                let communication = command.communication.clone();
                let result = crate::ui::utils::borg::exec(command, &quit_guard)
                    .await
                    .into_message(gettext("Verify Archives Integrity"));
                let mut message_history = communication
                    .general_info
                    .load()
                    .all_combined_message_history();

                // The actual error message is not very interesting, we need to dig through history
                if let Err(err) = result {
                    if message_history.is_empty() {
                        message_history = vec![LogEntry::UnparsableErr(err.to_string())];
                    }

                    if matches!(err, Error::UserCanceled) {
                        BACKUP_HISTORY.try_update(|history| {
                            history.set_last_check(config.id.clone(), CheckRunInfo::new_aborted());
                            Ok(())
                        })?;

                        return Ok(());
                    }
                }

                if !message_history.is_empty() {
                    let run_info = if repair {
                        crate::config::history::CheckRunInfo::new_repair(message_history.clone())
                    } else {
                        crate::config::history::CheckRunInfo::new_error(message_history.clone())
                    };

                    BACKUP_HISTORY.try_update(|history| {
                        history.set_last_check(config.id.clone(), run_info.clone());
                        Ok(())
                    })?;

                    return Err(Message::new(
                        gettext("Verify Archives Integrity"),
                        message_history
                            .iter()
                            .map(|h| h.message())
                            .collect::<Vec<String>>()
                            .join("\n"),
                    )
                    .into());
                } else {
                    let run_info = crate::config::history::CheckRunInfo::new_success();

                    BACKUP_HISTORY.try_update(|history| {
                        history.set_last_check(config.id.clone(), run_info.clone());
                        Ok(())
                    })?;

                    crate::ui::utils::show_notice(gettext("Verify archives integrity completed successfully"));
                }


                Ok(())
            }));
        }
    }
}

glib::wrapper! {
    pub struct CheckDialog(ObjectSubclass<imp::CheckDialog>)
        @extends gtk::Widget, gtk::Window, adw::Window;
}

impl CheckDialog {
    pub fn new(config_id: ConfigId) -> Self {
        glib::Object::builder()
            .property("config-id", config_id)
            .build()
    }
}
