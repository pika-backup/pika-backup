use adw::prelude::*;

use crate::borg;
use crate::config;
use crate::ui;
use crate::ui::prelude::*;

use adw::subclass::prelude::*;

mod imp {
    use self::borg::{ListArchive, PruneInfo};

    use super::*;
    use std::cell::RefCell;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(file = "prune_dialog.ui")]
    pub struct PruneDialog {
        #[template_child]
        stack: TemplateChild<gtk::Stack>,
        #[template_child]
        page_decision: TemplateChild<adw::ToolbarView>,

        #[template_child]
        delete: TemplateChild<gtk::Button>,
        #[template_child]
        cancel: TemplateChild<gtk::Button>,

        #[template_child]
        prune: TemplateChild<gtk::Label>,
        #[template_child]
        keep: TemplateChild<gtk::Label>,
        #[template_child]
        untouched: TemplateChild<gtk::Label>,

        result_sender: RefCell<Option<futures::channel::oneshot::Sender<bool>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PruneDialog {
        const NAME: &'static str = "PkPruneDialog";
        type Type = super::PruneDialog;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PruneDialog {}
    impl WidgetImpl for PruneDialog {}
    impl WindowImpl for PruneDialog {
        fn close_request(&self) -> glib::Propagation {
            if let Some(sender) = self.result_sender.take() {
                let _ignore = sender.send(false);
            }

            self.parent_close_request()
        }
    }
    impl AdwWindowImpl for PruneDialog {}

    #[gtk::template_callbacks]
    impl PruneDialog {
        pub(super) async fn delete(&self, config: &crate::config::Backup) -> Result<()> {
            self.obj().close();

            let guard = QuitGuard::default();
            let result = ui::utils::borg::exec(
                borg::Command::<borg::task::Prune>::new(config.clone()),
                &guard,
            )
            .await;

            if !result.is_borg_err_user_aborted() {
                result.into_message(gettext("Delete old Archives"))?;
            }

            let result = ui::utils::borg::exec(
                borg::Command::<borg::task::Compact>::new(config.clone()),
                &guard,
            )
            .await;

            if !result.is_borg_err_user_aborted() {
                result.into_message(gettext("Reclaim Free Space"))?;
            }

            let _ignore = main_ui()
                .page_detail()
                .archives_page()
                .refresh_archives(config.clone(), None)
                .await;
            let _ignore = ui::utils::df::lookup_and_cache(config).await;

            Ok(())
        }

        #[template_callback]
        fn on_delete(&self) {
            if let Some(sender) = self.result_sender.take() {
                let _ignore = sender.send(true);
            }

            self.obj().close();
        }

        pub(super) async fn choose_future(&self, config: &config::Backup) -> Result<()> {
            let guard = QuitGuard::default();

            let (sender, receiver) = futures::channel::oneshot::channel();
            self.result_sender.replace(Some(sender));

            let prune_info = ui::utils::borg::exec(
                borg::Command::<borg::task::PruneInfo>::new(config.clone()),
                &guard,
            )
            .await
            .into_message(gettext(
                "Failed to determine how many archives would be deleted",
            ))?;

            let list_all = ui::utils::borg::exec(
                borg::Command::<borg::task::List>::new(config.clone()),
                &guard,
            )
            .await
            .into_message("List Archives")?;

            self.set_prune_info(&prune_info, &list_all);

            if Ok(true) == receiver.await {
                Ok(())
            } else {
                Err(Error::UserCanceled)
            }
        }

        pub(super) fn set_prune_info(&self, prune_info: &PruneInfo, list_all: &[ListArchive]) {
            self.prune.set_label(&prune_info.prune.to_string());
            self.keep.set_label(&prune_info.keep.to_string());

            let num_untouched_archives = list_all.len() - prune_info.prune - prune_info.keep;
            self.untouched
                .set_label(&num_untouched_archives.to_string());

            if prune_info.prune == 0 {
                self.delete.set_visible(false);
                self.cancel.set_label(&gettext("Close"));
            }

            self.stack.set_visible_child(&*self.page_decision);
        }
    }
}

glib::wrapper! {
    pub struct PruneDialog(ObjectSubclass<imp::PruneDialog>)
    @extends gtk::Widget, gtk::Window, adw::Window,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl PruneDialog {
    fn new() -> Self {
        glib::Object::new()
    }

    pub async fn ask_prune(
        transient_for: &impl IsA<gtk::Window>,
        config: &config::Backup,
    ) -> Result<()> {
        // First ensure the device is available to prevent overlapping dialogs
        ui::dialog_device_missing::ensure_device_plugged_in(
            config,
            &gettext("Identifying old Archives"),
        )
        .await?;

        let dialog = Self::new();
        dialog.set_transient_for(Some(transient_for));
        dialog.present();

        // Returns Error::UserCanceled if canceled
        dialog.imp().choose_future(config).await?;

        // Run prune operation
        dialog.imp().delete(config).await?;

        Ok(())
    }
}
