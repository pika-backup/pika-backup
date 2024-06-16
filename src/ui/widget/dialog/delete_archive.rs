use crate::borg;
use crate::config;
use crate::ui;
use crate::ui::prelude::*;

use adw::prelude::*;
use adw::subclass::prelude::*;

mod imp {
    use self::ui::{error::HandleError, App};

    use super::*;
    use std::cell::OnceCell;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[template(file = "delete_archive.ui")]
    #[properties(wrapper_type = super::DeleteArchiveDialog)]
    pub struct DeleteArchiveDialog {
        #[property(get, set, construct_only)]
        config: OnceCell<crate::config::Backup>,
        #[property(get, set, construct_only)]
        archive_name: OnceCell<String>,
        #[property(get, set, construct_only)]
        archive_date: OnceCell<String>,

        #[template_child]
        name: TemplateChild<adw::ActionRow>,
        #[template_child]
        date: TemplateChild<adw::ActionRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DeleteArchiveDialog {
        const NAME: &'static str = "PkDeleteArchiveDialog";
        type Type = super::DeleteArchiveDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for DeleteArchiveDialog {
        fn constructed(&self) {
            self.parent_constructed();
            self.name.set_subtitle(self.archive_name.get().unwrap());
            self.date.set_subtitle(self.archive_date.get().unwrap());
        }
    }
    impl WidgetImpl for DeleteArchiveDialog {}
    impl AdwDialogImpl for DeleteArchiveDialog {}

    #[gtk::template_callbacks]
    impl DeleteArchiveDialog {
        async fn delete(&self) -> Result<()> {
            let config = self
                .config
                .get()
                .expect("construct_only variable should be set");

            let guard = QuitGuard::default();

            let mut command = borg::Command::<borg::task::Delete>::new(config.clone());
            command
                .task
                .set_archive_name(self.archive_name.get().cloned());
            let result = ui::utils::borg::exec(command, &guard).await;

            result.into_message(gettext("Delete Archive Failed"))?;

            ui::utils::borg::exec(
                borg::Command::<borg::task::Compact>::new(config.clone()),
                &guard,
            )
            .await
            .into_message("Reclaiming Free Space Failed")?;

            let _ = App::default()
                .main_window()
                .page_detail()
                .archives_page()
                .refresh_archives(config.clone(), None)
                .await;

            Ok(())
        }

        #[template_callback]
        async fn on_delete(&self) {
            self.obj().close();
            self.delete().await.handle_transient_for(&*self.obj()).await;
        }
    }
}

glib::wrapper! {
    pub struct DeleteArchiveDialog(ObjectSubclass<imp::DeleteArchiveDialog>)
    @extends gtk::Widget, adw::Dialog,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl DeleteArchiveDialog {
    pub fn new(config: &config::Backup, archive_name: String, archive_date: String) -> Self {
        glib::Object::builder()
            .property("config", config)
            .property("archive-name", archive_name)
            .property("archive-date", archive_date)
            .build()
    }
}
