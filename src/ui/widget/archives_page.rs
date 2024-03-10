pub mod cache;
mod display;
mod events;

use adw::prelude::*;
use adw::subclass::prelude::*;

use crate::{config, schedule, ui};
use ui::prelude::*;

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
    use self::ui::widget::StatusRow;

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(file = "archives_page.ui")]
    pub struct ArchivesPage {
        // Location
        #[template_child]
        pub(super) archives_location_icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) archives_location_title: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) archives_location_subtitle: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) archives_location_suffix_title: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) archives_location_suffix_subtitle: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) archives_fs_usage: TemplateChild<gtk::LevelBar>,

        // Prefix
        #[template_child]
        pub(super) archives_prefix: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) archives_prefix_edit: TemplateChild<gtk::Button>,

        // Cleanup
        #[template_child]
        pub(super) archives_cleanup: TemplateChild<adw::ActionRow>,

        // Integrity check
        #[template_child]
        pub(super) check_status: TemplateChild<StatusRow>,
        #[template_child]
        pub(super) archives_check_now: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) archives_check_abort: TemplateChild<gtk::Button>,

        // Archives list header suffix
        #[template_child]
        pub(super) archives_reloading_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) refresh_archives: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) archives_reloading_spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub(super) archives_eject_button: TemplateChild<gtk::Button>,

        // Archives list
        #[template_child]
        pub(super) archives_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) archive_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) archive_list_placeholder: TemplateChild<gtk::ListBox>,
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

            self.archives_prefix_edit.connect_clicked(
                glib::clone!(@weak obj => move |_| Handler::run(async move { obj.imp().edit_prefix().await })),
            );

            // Backup details
            self.check_status.connect_activated(|_| {
                if let Some(id) = &**ACTIVE_BACKUP_ID.load() {
                    let dialog = main_ui().dialog_check_result();
                    dialog.set_config_id(Some(id.clone()));
                    dialog.present();
                }
            });
            self.archives_check_now
                .connect_clicked(glib::clone!(@weak obj => move |_| Handler::run(async move { obj.imp().edit_prefix().await })));
            self.archives_check_abort.connect_clicked(|_| {
                Handler::run(async move { main_ui().page_backup().show_abort_dialog().await })
            });

            self.archives_cleanup
                .connect_activated(glib::clone!(@weak obj => move |_| Handler::run(async move { obj.imp().cleanup().await })));

            self.refresh_archives
                .connect_clicked(glib::clone!(@weak obj => move |_| {
                    Handler::run(async move {
                        let config = BACKUP_CONFIG.load().active()?.clone();
                        obj.imp().refresh_archives(config, None).await
                    });
                }));

            self.archives_eject_button
                .connect_clicked(glib::clone!(@weak obj => move |_| {
                    Handler::run(async move { obj.imp().eject_button_clicked().await });
                }));

            // spinner performance

            self.archives_reloading_spinner.connect_map(|s| s.start());
            self.archives_reloading_spinner.connect_unmap(|s| s.stop());

            glib::timeout_add_local(std::time::Duration::ZERO, move || {
                // TODO: This should be run directly, but as long as we need main_ui we need to do it later to prevent recursion
                main_ui().detail_stack().connect_visible_child_notify(
                    glib::clone!(@weak obj => move |_| {
                        if obj.is_visible() {
                            Handler::run(async move { obj.imp().show().await });
                        }
                    }),
                );

                glib::ControlFlow::Break
            });
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
        main_ui().detail_stack().visible_child()
            == Some(main_ui().page_archives().upcast::<gtk::Widget>())
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
