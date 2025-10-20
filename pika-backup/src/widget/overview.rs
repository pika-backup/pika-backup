mod row;

use std::collections::BTreeMap;

use adw::prelude::*;
use adw::subclass::prelude::*;

use crate::prelude::*;

mod imp {
    use std::cell::RefCell;

    use self::row::OverviewRow;
    use super::*;
    use crate::widget::setup::SetupDialog;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(file = "overview.ui")]
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

        rows: RefCell<BTreeMap<ConfigId, OverviewRow>>,
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
            let obj = self.obj();

            self.add_backup.connect_clicked(glib::clone!(
                #[weak]
                obj,
                move |_| {
                    let window = obj.app_window();
                    SetupDialog::new().present(Some(&window));
                }
            ));
            self.add_backup_empty.connect_clicked(glib::clone!(
                #[weak]
                obj,
                move |_| {
                    let window = obj.app_window();
                    SetupDialog::new().present(Some(&window));
                }
            ));

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
            crate::utils::clear(&self.main_backups);
            self.rows.borrow_mut().clear();

            for config in BACKUP_CONFIG.load().iter() {
                let row = OverviewRow::new(config);
                self.main_backups.append(&row);
                self.rows.borrow_mut().insert(config.id.clone(), row);
            }

            self.force_refresh_status();
        }

        pub(super) fn force_refresh_status(&self) {
            let imp = self.ref_counted();
            glib::MainContext::default().spawn_local(async move {
                for config in BACKUP_CONFIG.load().iter() {
                    // TODO: This should be a pure data object like backup_status::Display
                    let schedule_status = crate::widget::ScheduleStatus::new(config).await;
                    let rows = imp.rows.borrow();
                    if let Some(row) = rows.get(&config.id) {
                        let status = crate::backup_status::Display::new_from_id(&config.id);

                        row.status().set_from_backup_status(&status);
                        // schedule status

                        row.schedule_status()
                            .set_title(&glib::markup_escape_text(&schedule_status.main.title()));
                        row.schedule_status()
                            .set_subtitle(&glib::markup_escape_text(
                                &schedule_status.main.subtitle().unwrap_or_default(),
                            ));
                        row.schedule_status()
                            .set_icon_name(schedule_status.main.icon_name());
                        row.schedule_status()
                            .set_level(schedule_status.main.level());
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
        crate::utils::confirmation_dialog(
            self,
            &gettext("Remove Backup Setup?"),
            &gettext("Removing the setup will not delete any archives"),
            &gettext("_Cancel"),
            &gettext("_Remove Setup"),
        )
        .await?;

        let config = BACKUP_CONFIG.load().active()?.clone();

        let config_id = config.id.clone();

        BACKUP_CONFIG
            .try_update(|s| {
                s.remove(&config_id)?;
                Ok(())
            })
            .await?;

        if let Err(err) = crate::utils::password_storage::remove_password(&config, false).await {
            // Display the error and continue to leave the UI in a consistent state
            err.show().await;
        }

        ACTIVE_BACKUP_ID.update(|active_id| *active_id = None);

        self.imp().reload_visible_page();
        main_ui().navigation_view().pop_to_page(self);

        Ok(())
    }
}

impl HasAppWindow for OverviewPage {
    fn app_window(&self) -> super::AppWindow {
        self.root()
            .and_downcast()
            .expect("PkOverviewPage must be inside PkAppWindow")
    }
}
