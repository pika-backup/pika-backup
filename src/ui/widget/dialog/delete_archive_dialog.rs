use crate::borg;
use crate::config;
use crate::ui;
use crate::ui::prelude::*;

use adw::prelude::*;
use adw::subclass::prelude::*;

mod imp {
    use self::ui::{error::HandleError, App};

    use super::*;
    use std::cell::{OnceCell, RefCell};

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[template(file = "delete_archive_dialog.ui")]
    #[properties(wrapper_type = super::DeleteArchiveDialog)]
    pub struct DeleteArchiveDialog {
        #[property(get, set, construct_only)]
        config: OnceCell<crate::config::Backup>,
        archive_name: RefCell<String>,

        #[template_child]
        name: TemplateChild<adw::ActionRow>,
        #[template_child]
        date: TemplateChild<adw::ActionRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DeleteArchiveDialog {
        const NAME: &'static str = "PkDeleteArchiveDialog";
        type Type = super::DeleteArchiveDialog;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for DeleteArchiveDialog {}
    impl WidgetImpl for DeleteArchiveDialog {}
    impl WindowImpl for DeleteArchiveDialog {}
    impl AdwWindowImpl for DeleteArchiveDialog {}

    #[gtk::template_callbacks]
    impl DeleteArchiveDialog {
        pub(super) fn set_archive(&self, archive_name: &str, archive_date: &str) {
            self.name.set_subtitle(archive_name);
            self.archive_name.replace(archive_name.to_string());

            self.date.set_subtitle(archive_date);
        }

        async fn delete(&self) -> Result<()> {
            let config = self
                .config
                .get()
                .expect("construct_only variable should be set");

            let guard = QuitGuard::default();
            let archive_name = self.archive_name.borrow().clone();

            let mut command = borg::Command::<borg::task::Delete>::new(config.clone());
            command.task.set_archive_name(Some(archive_name.clone()));
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
            self.delete()
                .await
                .handle_transient_for(self.obj().transient_for().as_ref())
                .await;
        }
    }
}

glib::wrapper! {
    pub struct DeleteArchiveDialog(ObjectSubclass<imp::DeleteArchiveDialog>)
    @extends gtk::Widget, gtk::Window, adw::Window,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl DeleteArchiveDialog {
    pub fn new(config: &config::Backup) -> Self {
        glib::Object::builder().property("config", config).build()
    }

    pub fn present_with_archive(
        &self,
        transient_for: &impl IsA<gtk::Window>,
        archive_name: &str,
        archive_date: &str,
    ) {
        self.set_transient_for(Some(transient_for));
        self.present();
        self.imp().set_archive(archive_name, archive_date);
    }
}
