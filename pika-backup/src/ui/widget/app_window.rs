use adw::prelude::*;
use adw::subclass::prelude::*;

use super::detail::DetailPageKind;
use super::{DetailPage, OverviewPage};
use crate::ui::prelude::*;
use crate::ui::{self, App};

mod imp {
    use self::ui::widget::{DetailPage, OverviewPage};
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(file = "app_window.ui")]
    pub struct AppWindow {
        #[template_child]
        pub(super) toast: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub(super) navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub(super) page_overview: TemplateChild<OverviewPage>,
        #[template_child]
        pub(super) page_detail: TemplateChild<DetailPage>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AppWindow {
        const NAME: &'static str = "PkAppWindow";
        type Type = super::AppWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AppWindow {
        fn constructed(&self) {
            debug!("AppWindow::constructed");
            self.parent_constructed();

            // decorate headerbar of pre-release versions
            if !option_env!("APPLICATION_ID_SUFFIX")
                .unwrap_or_default()
                .is_empty()
            {
                self.obj().add_css_class("devel");
            }

            let imp = self.ref_counted();
            self.navigation_view
                .connect_visible_page_notify(move |navigation_view| {
                    if navigation_view.visible_page().is_some_and(|page| {
                        &page == imp.page_overview.upcast_ref::<adw::NavigationPage>()
                    }) {
                        imp.page_overview.reload_visible_page();
                    }
                });
        }
    }

    impl WidgetImpl for AppWindow {
        fn map(&self) {
            debug!("AppWindow::map");
            self.parent_map();

            // When launching the app and we only have one backup config, display it
            // instead of the overview
            if self.navigation_view.visible_page().as_ref() == Some(self.page_overview.upcast_ref())
                && BACKUP_CONFIG.load().iter().count() == 1
                && let Some(config) = BACKUP_CONFIG.load().iter().next()
            {
                self.navigation_view.replace(&[
                    self.page_overview.clone().upcast(),
                    self.page_detail.clone().upcast(),
                ]);
                self.obj().view_backup_conf(&config.id);
            }

            Handler::run(ui::init_check_borg());

            // redo size estimates for backups running in background before
            BORG_OPERATION.with(|operations| {
                for (config_id, operation) in operations.load().iter() {
                    if let Some(create_op) = operation.try_as_create()
                        && create_op
                            .communication()
                            .specific_info
                            .load()
                            .estimated_size
                            .is_none()
                    {
                        debug!("A running backup is lacking size estimate");
                        if let Some(config) = BACKUP_CONFIG.load().try_get(config_id).ok().cloned()
                        {
                            let communication = create_op.communication().clone();
                            glib::MainContext::default().spawn_local(async move {
                                ui::toast_size_estimate::check(&config, communication).await
                            });
                        }
                    }
                }
            });
        }
    }

    impl WindowImpl for AppWindow {
        fn close_request(&self) -> glib::Propagation {
            debug!("AppWindow::close_request");
            let app = self.app();

            Handler::run(async move { app.try_quit().await });
            glib::Propagation::Stop
        }
    }
    impl ApplicationWindowImpl for AppWindow {}
    impl AdwApplicationWindowImpl for AppWindow {}

    #[gtk::template_callbacks]
    impl AppWindow {
        fn app(&self) -> App {
            self.obj().app()
        }
    }
}

glib::wrapper! {
    pub struct AppWindow(ObjectSubclass<imp::AppWindow>)
    @extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
    @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl AppWindow {
    pub fn new(app: &App) -> Self {
        debug!("Creating new PkAppWindow");
        glib::Object::builder().property("application", app).build()
    }

    pub fn app(&self) -> App {
        self.application()
            .and_downcast()
            .expect("Application must be PkApp subclass")
    }

    pub fn window(&self) -> Self {
        self.clone()
    }

    pub fn toast(&self) -> adw::ToastOverlay {
        self.imp().toast.clone()
    }

    pub fn navigation_view(&self) -> adw::NavigationView {
        self.imp().navigation_view.clone()
    }

    pub fn page_overview(&self) -> OverviewPage {
        self.imp().page_overview.clone()
    }

    pub fn page_detail(&self) -> DetailPage {
        self.imp().page_detail.clone()
    }

    pub fn view_backup_conf(&self, id: &ConfigId) {
        let imp = self.imp();
        ACTIVE_BACKUP_ID.update(|active_id| *active_id = Some(id.clone()));

        imp.page_detail.show_stack_page(DetailPageKind::Backup);

        if imp.navigation_view.visible_page().as_ref() != Some(imp.page_detail.upcast_ref()) {
            imp.navigation_view.push(&main_ui().page_detail());
            imp.page_detail.grab_focus();
        }
    }
}
