mod display;
mod events;
mod execution;

use adw::prelude::*;
use adw::subclass::prelude::*;
use common::{borg, schedule};

use super::DetailPageKind;
use crate::prelude::*;
use crate::widget::dialog::SizeEstimateDialog;

mod imp {
    use std::cell::RefCell;

    use super::*;
    use crate::backup_status;
    use crate::widget::dialog::{BackupInfoDialog, StorageDialog};

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(file = "backup.ui")]
    pub struct BackupPage {
        /// The last known backup status
        pub(super) backup_status: RefCell<Option<backup_status::Display>>,

        #[template_child]
        pub(super) detail_dialog: TemplateChild<BackupInfoDialog>,

        // status section
        #[template_child]
        pub(super) detail_repo_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) detail_repo_icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) detail_status_row: TemplateChild<crate::widget::StatusRow>,
        #[template_child]
        pub(super) detail_hint_icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) backup_disk_eject_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) backup_disk_disconnected: TemplateChild<gtk::Box>,

        // create and abort buttons
        #[template_child]
        pub(super) backup_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) abort_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) size_estimate_button: TemplateChild<gtk::Button>,

        // lists
        #[template_child]
        pub(super) include_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) add_include_file_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) add_include_folder_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) exclude_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) add_exclude_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BackupPage {
        const NAME: &'static str = "PkBackupPage";
        type Type = super::BackupPage;
        type ParentType = adw::PreferencesPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for BackupPage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj().clone();

            self.backup_button.connect_clicked(glib::clone!(
                #[weak]
                obj,
                move |_| {
                    let guard = QuitGuard::default();
                    Handler::run(async move { obj.imp().on_backup_run(&guard).await });
                }
            ));

            self.size_estimate_button.connect_clicked(glib::clone!(
                #[weak]
                obj,
                move |_| {
                    //let guard = QuitGuard::default();
                    //Handler::run(async move { obj.imp().on_backup_run(&guard).await });
                    let dialog = SizeEstimateDialog::new();
                    dialog.present(Some(&obj));

                    Handler::run(async move {
                        let config = BACKUP_CONFIG.load().active()?.clone();
                        let history = BACKUP_HISTORY.load().clone();
                        let result = borg::size_estimate::calculate(
                            &config,
                            &history,
                            &borg::Communication::default(),
                            5,
                        )
                        .unwrap();
                        dialog.set_data(result);
                        Ok(())
                    });
                }
            ));

            // Backup details
            self.detail_status_row.connect_activated(glib::clone!(
                #[weak]
                obj,
                move |_| {
                    let imp = obj.imp();
                    imp.detail_dialog.present(Some(&obj));

                    if let Some(status) = &*imp.backup_status.borrow() {
                        imp.detail_dialog.refresh_status_display(status);
                    };
                }
            ));

            self.detail_repo_row.connect_activated(glib::clone!(
                #[weak]
                obj,
                move |_| {
                    let window = obj.app_window();
                    Handler::run(async move {
                        let dialog = StorageDialog::new(BACKUP_CONFIG.load().active()?).await;
                        dialog.present(Some(&window));
                        Ok(())
                    });
                }
            ));

            self.add_include_file_button.connect_clicked(glib::clone!(
                #[weak]
                obj,
                move |_| Handler::run(async move { obj.imp().add_include_file().await })
            ));
            self.add_include_folder_button.connect_clicked(glib::clone!(
                #[weak]
                obj,
                move |_| Handler::run(async move { obj.imp().add_include().await })
            ));
            self.add_exclude_button.connect_clicked(glib::clone!(
                #[weak]
                obj,
                move |_| Handler::run(async move { obj.imp().add_exclude().await })
            ));

            self.abort_button.connect_clicked(glib::clone!(
                #[weak]
                obj,
                move |_| Handler::run(async move { obj.imp().on_stop_backup_create().await })
            ));

            self.backup_disk_eject_button.connect_clicked(glib::clone!(
                #[weak]
                obj,
                move |_| Handler::run(async move { obj.imp().on_backup_disk_eject().await })
            ));

            glib::timeout_add_local(std::time::Duration::ZERO, move || {
                // TODO: This should be run directly, but as long as we need main_ui we need to
                // do it later to prevent recursion
                main_ui()
                    .navigation_view()
                    .connect_visible_page_notify(glib::clone!(
                        #[weak]
                        obj,
                        move |navigation_view| {
                            if navigation_view
                                .visible_page()
                                .is_some_and(|page| page == main_ui().page_detail())
                            {
                                Handler::handle(obj.imp().refresh());
                            }
                        }
                    ));

                glib::ControlFlow::Break
            });
        }
    }

    impl WidgetImpl for BackupPage {
        fn grab_focus(&self) -> bool {
            self.backup_button.grab_focus()
        }
    }

    impl PreferencesPageImpl for BackupPage {}

    impl BackupPage {}
}

glib::wrapper! {
    pub struct BackupPage(ObjectSubclass<imp::BackupPage>)
    @extends adw::PreferencesPage, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl BackupPage {
    pub fn start_backup(
        &self,
        id: ConfigId,
        due_cause: Option<schedule::DueCause>,
        guard: QuitGuard,
    ) {
        let obj = self.clone();

        // We spawn a new task instead of waiting for backup completion here.
        //
        // This is necessary because we can start backups from many different sources,
        // including dbus. If we waited here we wouldn't be receiving any more
        // dbus messages until this backup is finished.
        Handler::run(async move {
            obj.imp()
                .backup(
                    BACKUP_CONFIG.load().try_get(&id)?.clone(),
                    due_cause,
                    &guard,
                )
                .await
        });
    }

    fn is_visible(&self) -> bool {
        main_ui().page_detail().visible_stack_page() == DetailPageKind::Backup
    }

    /// Shows the dialog to abort a running backup operation.
    ///
    /// Aborts the operation if successful.
    pub async fn show_abort_dialog(&self) -> Result<()> {
        self.imp().on_stop_backup_create().await
    }

    pub fn refresh_disk_status(&self) {
        // TODO: This doesn't need to be public, replace with signals
        self.imp().refresh_disk_status();
    }

    pub fn refresh(&self) -> Result<()> {
        self.imp().refresh()
    }

    pub fn refresh_status(&self) {
        // TODO: This doesn't need to be public, replace with signals
        self.imp().refresh_status();
    }
}

impl HasAppWindow for BackupPage {
    fn app_window(&self) -> crate::widget::AppWindow {
        self.root()
            .and_downcast()
            .expect("PkBackupPage must be inside PkAppWindow")
    }
}
