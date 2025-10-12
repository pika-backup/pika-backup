use adw::prelude::*;

use crate::borg;
use crate::config;
use crate::ui;
use crate::ui::prelude::*;

use adw::subclass::prelude::*;

mod imp {
    use adw::subclass::dialog::AdwDialogImplExt;

    use self::borg::{ListArchive, PruneInfo};

    use super::*;
    use std::cell::{Cell, OnceCell, RefCell};

    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::PruneDialog)]
    #[template(file = "prune.ui")]
    pub struct PruneDialog {
        #[property(get, set, construct_only)]
        config: OnceCell<config::Backup>,
        #[property(get, set, construct_only)]
        review_only: Cell<bool>,

        #[template_child]
        stack: TemplateChild<gtk::Stack>,
        #[template_child]
        status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        page_decision: TemplateChild<adw::ToolbarView>,

        #[template_child]
        apply_button: TemplateChild<gtk::Button>,
        #[template_child]
        delete_button: TemplateChild<gtk::Button>,

        #[template_child]
        preferences_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        prune: TemplateChild<gtk::Label>,
        #[template_child]
        keep: TemplateChild<gtk::Label>,
        #[template_child]
        untouched: TemplateChild<gtk::Label>,

        result_sender: RefCell<Option<futures_channel::oneshot::Sender<bool>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PruneDialog {
        const NAME: &'static str = "PkPruneDialog";
        type Type = super::PruneDialog;
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
    impl ObjectImpl for PruneDialog {
        fn constructed(&self) {
            self.parent_constructed();

            if self.review_only.get() {
                self.obj().set_title(&gettext("Review Changes"))
            }

            let status = if self.review_only.get() {
                gettext("Collecting information about the effect of the changes on old archives")
            } else {
                gettext("Creating a list of old archives to be approved for deletion")
            };

            self.status_page.set_description(Some(&status));

            let description = if self.review_only.get() {
                gettext(
                    "After applying these changes, the next automatic deletion of old archives would have the following consequences.",
                )
            } else {
                gettext(
                    "Proceeding with this operation will irretrievably delete some of the archives. The saved data for those specific points in time will no longer be available.",
                )
            };

            self.preferences_group.set_description(Some(&description));
            self.apply_button.set_visible(self.review_only.get());
            self.delete_button.set_visible(!self.review_only.get());
        }
    }
    impl WidgetImpl for PruneDialog {}
    impl AdwDialogImpl for PruneDialog {
        fn closed(&self) {
            if let Some(sender) = self.result_sender.take() {
                let _ignore = sender.send(false);
            }

            self.parent_closed();
        }
    }

    #[gtk::template_callbacks]
    impl PruneDialog {
        pub(super) async fn delete(&self, config: &crate::config::Backup) -> Result<()> {
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
        fn on_apply(&self) {
            if let Some(sender) = self.result_sender.take() {
                let _ignore = sender.send(true);
            }

            self.obj().close();
        }

        pub(super) async fn choose_future(&self, config: &config::Backup) -> Result<()> {
            let guard = QuitGuard::default();

            let (sender, receiver) = futures_channel::oneshot::channel();
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

            if prune_info.prune == 0 && !self.review_only.get() {
                self.delete_button.set_visible(false);
                self.preferences_group.set_description(Some(&gettext(
                    "No archives are selected for deletion with current cleanup rules",
                )));
            }

            self.stack.set_visible_child(&*self.page_decision);
        }
    }
}

glib::wrapper! {
    pub struct PruneDialog(ObjectSubclass<imp::PruneDialog>)
    @extends gtk::Widget, adw::Dialog,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl PruneDialog {
    pub fn new(config: config::Backup, review_only: bool) -> Self {
        glib::Object::builder()
            .property("config", config)
            .property("review-only", review_only)
            .build()
    }

    pub async fn execute(&self, parent: &impl IsA<gtk::Widget>) -> Result<()> {
        let config = self.config();

        // First ensure the device is available to prevent overlapping dialogs
        ui::repo::ensure_device_plugged_in(
            parent.upcast_ref(),
            &config,
            &gettext("Identifying old Archives"),
        )
        .await?;

        self.present(Some(parent));

        // Returns Error::UserCanceled if canceled
        self.imp().choose_future(&config).await?;

        // Run prune operation
        if !self.review_only() {
            self.imp().delete(&config).await?;
        }

        Ok(())
    }
}
