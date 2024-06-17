use adw::prelude::*;

use crate::borg;
use crate::config;
use crate::ui;
use crate::ui::prelude::*;

use adw::subclass::prelude::*;

mod imp {
    use std::cell::RefCell;

    use adw::subclass::dialog::AdwDialogImplExt;

    use self::borg::{ListArchive, PruneInfo};

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(file = "prune_review.ui")]
    pub struct PruneReviewDialog {
        #[template_child]
        stack: TemplateChild<gtk::Stack>,
        #[template_child]
        page_decision: TemplateChild<adw::ToolbarView>,

        #[template_child]
        prune: TemplateChild<gtk::Label>,
        #[template_child]
        keep: TemplateChild<gtk::Label>,
        #[template_child]
        untouched: TemplateChild<gtk::Label>,

        result_sender: RefCell<Option<futures::channel::oneshot::Sender<bool>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PruneReviewDialog {
        const NAME: &'static str = "PkPruneReviewDialog";
        type Type = super::PruneReviewDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PruneReviewDialog {}
    impl WidgetImpl for PruneReviewDialog {}
    impl AdwDialogImpl for PruneReviewDialog {
        fn closed(&self) {
            if let Some(sender) = self.result_sender.take() {
                let _ignore = sender.send(false);
            }

            self.parent_closed()
        }
    }

    #[gtk::template_callbacks]
    impl PruneReviewDialog {
        #[template_callback]
        fn on_apply(&self) {
            if let Some(sender) = self.result_sender.take() {
                let _ignore = sender.send(true);
            }

            self.obj().close();
        }

        pub(super) async fn choose_future(&self, config: &crate::config::Backup) -> Result<()> {
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

            self.stack.set_visible_child(&*self.page_decision);
        }
    }
}

glib::wrapper! {
    pub struct PruneReviewDialog(ObjectSubclass<imp::PruneReviewDialog>)
    @extends gtk::Widget, adw::Dialog,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl PruneReviewDialog {
    fn new() -> Self {
        glib::Object::new()
    }

    pub async fn present(parent: &impl IsA<gtk::Widget>, config: &config::Backup) -> Result<()> {
        // First ensure the device is available to prevent overlapping dialogs
        ui::repo::ensure_device_plugged_in(
            parent.upcast_ref(),
            config,
            &gettext("Identifying old Archives"),
        )
        .await?;

        let dialog = PruneReviewDialog::new();
        dialog.present(parent);
        dialog.imp().choose_future(config).await
    }
}
