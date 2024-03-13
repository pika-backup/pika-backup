use crate::ui;
use crate::ui::prelude::*;

use std::collections::BTreeMap;

use adw::prelude::*;
use adw::subclass::prelude::*;

mod imp {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(file = "overview_page.ui")]
    pub struct OverviewPage {
        #[template_child]
        pub(super) add_backup: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) primary_menu_button: TemplateChild<gtk::MenuButton>,

        #[template_child]
        pub(super) main_stack: TemplateChild<adw::ViewStack>,
        #[template_child]
        pub(super) page_overview_empty: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub(super) add_backup_empty: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) page_overview: TemplateChild<adw::PreferencesPage>,
        #[template_child]
        pub(super) main_backups: TemplateChild<gtk::ListBox>,

        rows: RefCell<BTreeMap<ConfigId, ui::builder::OverviewItem>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for OverviewPage {
        const NAME: &'static str = "PkOverviewPage";
        type Type = super::OverviewPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for OverviewPage {
        fn constructed(&self) {
            self.parent_constructed();

            self.add_backup
                .connect_clicked(|_| ui::dialog_setup::show());
            self.add_backup_empty
                .connect_clicked(|_| ui::dialog_setup::show());

            let imp = self.ref_counted();
            self.main_backups.connect_map(move |_| imp.rebuild_list());
            self.reload_visible_page();
        }
    }

    impl WidgetImpl for OverviewPage {}
    impl NavigationPageImpl for OverviewPage {}

    #[gtk::template_callbacks]
    impl OverviewPage {
        pub fn reload_visible_page(&self) {
            if BACKUP_CONFIG.load().iter().next().is_none() {
                self.main_stack
                    .set_visible_child(&*self.page_overview_empty);
            } else {
                self.main_stack.set_visible_child(&*self.page_overview);
            };
        }

        fn rebuild_list(&self) {
            ui::utils::clear(&self.main_backups);
            self.rows.borrow_mut().clear();

            for config in BACKUP_CONFIG.load().iter() {
                let row = ui::builder::OverviewItem::new();
                self.main_backups.append(&row.widget());

                // connect click
                row.location()
                    .connect_activated(enclose!((config) move |_| {
                        main_ui().view_backup_conf(&config.id);
                    }));

                row.schedule()
                    .connect_activated(enclose!((config) move |_| {
                        main_ui().page_detail().schedule_page().view(&config.id);
                    }));

                // Repo Icon

                if let Ok(icon) = gio::Icon::for_string(&config.repo.icon()) {
                    row.location_icon().set_from_gicon(&icon);
                }

                // Repo Name

                row.location_title().set_label(&config.title());
                row.location_subtitle().set_label(&config.repo.subtitle());

                // Include

                for path in &config.include {
                    let incl = ui::widget::LocationTag::from_path(path.clone());

                    row.include().add_child(&incl.build());
                }

                self.rows.borrow_mut().insert(config.id.clone(), row);
            }

            self.force_refresh_status();
        }

        pub(super) fn force_refresh_status(&self) {
            let imp = self.ref_counted();
            glib::MainContext::default().spawn_local(async move {
                for config in BACKUP_CONFIG.load().iter() {
                    let schedule_status = ui::widget::Status::new(config).await;
                    let rows = imp.rows.borrow();
                    if let Some(row) = rows.get(&config.id) {
                        let status = ui::backup_status::Display::new_from_id(&config.id);

                        row.status().set_from_backup_status(&status);
                        // schedule status

                        row.schedule()
                            .set_title(&glib::markup_escape_text(&schedule_status.main.title()));
                        row.schedule().set_subtitle(&glib::markup_escape_text(
                            &schedule_status.main.subtitle().unwrap_or_default(),
                        ));
                        row.schedule()
                            .set_icon_name(schedule_status.main.icon_name());
                        row.schedule().set_level(schedule_status.main.level());
                    }
                }
            });
        }
    }
}

glib::wrapper! {
    pub struct OverviewPage(ObjectSubclass<imp::OverviewPage>)
    @extends adw::NavigationPage, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl OverviewPage {
    pub fn dbus_show(&self) {
        self.imp()
            .main_stack
            .set_visible_child(&main_ui().page_overview());
        adw_app().activate();
    }

    pub fn refresh_status(&self) {
        if self.is_visible() {
            self.imp().force_refresh_status();
        }
    }

    pub fn reload_visible_page(&self) {
        self.imp().reload_visible_page();
    }

    fn is_visible(&self) -> bool {
        self.imp().main_stack.visible_child()
            == Some(main_ui().page_overview().upcast::<gtk::Widget>())
    }

    pub fn remove_backup(&self) {
        let obj = self.clone();
        Handler::run(async move { obj.on_remove_backup().await });
    }

    async fn on_remove_backup(&self) -> Result<()> {
        ui::utils::confirmation_dialog(
            &gettext("Remove Backup Setup?"),
            &gettext("Removing the setup will not delete any archives."),
            &gettext("Cancel"),
            &gettext("Remove Setup"),
        )
        .await?;

        let config = BACKUP_CONFIG.load().active()?.clone();

        let config_id = config.id.clone();

        BACKUP_CONFIG.try_update(|s| {
            s.remove(&config_id)?;
            Ok(())
        })?;

        if let Err(err) = ui::utils::password_storage::remove_password(&config, false).await {
            // Display the error and continue to leave the UI in a consistent state
            err.show().await;
        }

        ACTIVE_BACKUP_ID.update(|active_id| *active_id = None);

        self.imp().reload_visible_page();
        main_ui().navigation_view().pop_to_page(self);

        Ok(())
    }
}
