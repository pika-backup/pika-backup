pub mod cache;
mod display;
mod events;

use adw::prelude::*;
use adw::subclass::prelude::*;

use crate::{config, schedule, ui};
use ui::prelude::*;

use super::DetailPageKind;

fn find_first_populated_dir(dir: &std::path::Path) -> std::path::PathBuf {
    if let Ok(mut dir_iter) = dir.read_dir() {
        if let Some(Ok(new_dir)) = dir_iter.next() {
            if new_dir.path().is_dir() && dir_iter.next().is_none() {
                return find_first_populated_dir(&new_dir.path());
            }
        }
    }

    dir.to_path_buf()
}

mod imp {
    use self::ui::widget::{dialog::CheckResultDialog, StatusRow};

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(file = "archives.ui")]
    pub struct ArchivesPage {
        #[template_child]
        pub(super) check_result_dialog: TemplateChild<CheckResultDialog>,

        // Location
        #[template_child]
        pub(super) location_icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) location_title: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) location_subtitle: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) location_suffix_title: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) location_suffix_subtitle: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) fs_usage: TemplateChild<gtk::LevelBar>,

        // Prefix
        #[template_child]
        pub(super) prefix_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) prefix_edit_button: TemplateChild<gtk::Button>,

        // Cleanup
        #[template_child]
        pub(super) cleanup_row: TemplateChild<adw::ActionRow>,

        // Integrity check
        #[template_child]
        pub(super) check_status_row: TemplateChild<StatusRow>,
        #[template_child]
        pub(super) check_button_row: TemplateChild<adw::ButtonRow>,
        #[template_child]
        pub(super) check_abort_button_row: TemplateChild<adw::ButtonRow>,

        // Archives list header suffix
        #[template_child]
        pub(super) reloading_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) refresh_archives_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) reloading_spinner: TemplateChild<adw::Spinner>,
        #[template_child]
        pub(super) eject_button: TemplateChild<gtk::Button>,

        // Archives list
        #[template_child]
        pub(super) list_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) list_placeholder: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ArchivesPage {
        const NAME: &'static str = "PkArchivesPage";
        type Type = super::ArchivesPage;
        type ParentType = adw::PreferencesPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ArchivesPage {
        fn constructed(&self) {
            let obj = self.obj().clone();

            self.prefix_edit_button.connect_clicked(glib::clone!(
                #[weak]
                obj,
                move |_| Handler::run(async move { obj.imp().edit_prefix().await })
            ));

            // Backup details
            self.check_status_row.connect_activated(glib::clone!(
                #[weak]
                obj,
                move |_| {
                    if let Some(id) = &**ACTIVE_BACKUP_ID.load() {
                        let dialog = &obj.imp().check_result_dialog;
                        dialog.set_config_id(Some(id.clone()));
                        dialog.present(Some(&obj));
                    }
                }
            ));
            self.check_button_row.connect_activated(glib::clone!(
                #[weak]
                obj,
                move |_| Handler::run(async move { obj.imp().check().await })
            ));
            self.check_abort_button_row.connect_activated(|_| {
                Handler::run(async move {
                    main_ui()
                        .page_detail()
                        .backup_page()
                        .show_abort_dialog()
                        .await
                })
            });

            self.cleanup_row.connect_activated(glib::clone!(
                #[weak]
                obj,
                move |_| Handler::run(async move { obj.imp().cleanup().await })
            ));

            self.refresh_archives_button.connect_clicked(glib::clone!(
                #[weak]
                obj,
                move |_| {
                    Handler::run(async move {
                        let config = BACKUP_CONFIG.load().active()?.clone();
                        obj.imp().refresh_archives(config, None).await
                    });
                }
            ));

            self.eject_button.connect_clicked(glib::clone!(
                #[weak]
                obj,
                move |_| {
                    Handler::run(async move { obj.imp().eject_button_clicked().await });
                }
            ));
        }
    }

    impl WidgetImpl for ArchivesPage {}
    impl PreferencesPageImpl for ArchivesPage {}

    #[gtk::template_callbacks]
    impl ArchivesPage {}
}

glib::wrapper! {
    pub struct ArchivesPage(ObjectSubclass<imp::ArchivesPage>)
    @extends adw::PreferencesPage, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ArchivesPage {
    fn is_visible(&self) -> bool {
        main_ui().page_detail().visible_stack_page() == DetailPageKind::Archives
    }

    pub fn refresh(&self) {
        let obj = self.clone();
        Handler::run(async move { obj.imp().show().await });
    }

    pub async fn refresh_archives(
        &self,
        config: config::Backup,
        from_schedule: Option<schedule::DueCause>,
    ) -> Result<()> {
        // TODO: This doesn't need to be public, replace with signals
        self.imp().refresh_archives(config, from_schedule).await
    }

    pub fn refresh_status(&self) {
        // TODO: This doesn't need to be public, replace with signals
        self.imp().refresh_status();
    }

    pub fn update_info(&self, config: &config::Backup) {
        // TODO: This doesn't need to be public, replace with signals
        self.imp().update_info(config)
    }
}

impl HasAppWindow for ArchivesPage {
    fn app_window(&self) -> crate::ui::widget::AppWindow {
        self.root()
            .and_downcast()
            .expect("PkArchivesPage must be inside PkAppWindow")
    }
}
