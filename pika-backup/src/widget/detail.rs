mod archives;
mod backup;
mod schedule;

use adw::prelude::*;
use adw::subclass::prelude::*;
pub use archives::ArchivesPage;
pub use backup::BackupPage;
pub use schedule::status::Status as ScheduleStatus;
pub use schedule::{SchedulePage, frequency, prune_preset, weekday};

use crate::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailPageKind {
    Backup,
    Archives,
    Schedule,
}

mod imp {
    use std::cell::Cell;

    use super::*;
    use crate::widget::{ArchivesPage, BackupPage, SchedulePage};

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(file = "detail.ui")]
    pub struct DetailPage {
        #[template_child]
        pub(super) pending_menu: TemplateChild<gtk::MenuButton>,

        #[template_child]
        pub(super) detail_stack: TemplateChild<adw::ViewStack>,
        #[template_child]
        pub(super) page_backup: TemplateChild<BackupPage>,
        #[template_child]
        pub(super) page_archives: TemplateChild<ArchivesPage>,
        #[template_child]
        pub(super) page_schedule: TemplateChild<SchedulePage>,
        showing: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DetailPage {
        const NAME: &'static str = "PkDetailPage";
        type Type = super::DetailPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DetailPage {
        fn constructed(&self) {
            self.parent_constructed();
            let imp = self.ref_counted();

            glib::timeout_add_local_once(std::time::Duration::ZERO, move || {
                // TODO: This should be run directly, but as long as we need main_ui we need to
                // do it later to prevent recursion
                imp.on_visible_child_notify();
                main_ui().navigation_view().connect_pushed(glib::clone!(
                    #[weak]
                    imp,
                    move |view| imp.on_pushed(view)
                ));
            });
        }
    }

    impl WidgetImpl for DetailPage {
        fn grab_focus(&self) -> bool {
            match self.detail_stack.visible_child() {
                Some(child) => child.grab_focus(),
                _ => self.parent_grab_focus(),
            }
        }
    }
    impl NavigationPageImpl for DetailPage {
        fn hidden(&self) {
            self.showing.set(false);
        }

        fn showing(&self) {
            self.showing.set(true);
        }
    }

    #[gtk::template_callbacks]
    impl DetailPage {
        fn on_pushed(&self, navigation_view: &adw::NavigationView) {
            if navigation_view.visible_page().as_ref() == Some(self.obj().upcast_ref()) {
                for page in &[
                    self.page_backup.upcast_ref::<adw::PreferencesPage>(),
                    self.page_archives.upcast_ref(),
                    self.page_schedule.upcast_ref(),
                ] {
                    page.scroll_to_top();
                }
            }
        }

        #[template_callback]
        fn on_visible_child_notify(&self) {
            if self.showing.get() {
                let visible_page = self.detail_stack.visible_child();
                if let Some(backup) = visible_page.and_downcast_ref::<BackupPage>() {
                    Handler::handle(backup.refresh());
                } else if let Some(archives) = visible_page.and_downcast_ref::<ArchivesPage>() {
                    archives.refresh()
                } else if let Some(schedule) = visible_page.and_downcast_ref::<SchedulePage>() {
                    schedule.refresh();
                }
            }
        }
    }
}

glib::wrapper! {
    pub struct DetailPage(ObjectSubclass<imp::DetailPage>)
    @extends adw::NavigationPage, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl DetailPage {
    pub fn visible_stack_page(&self) -> DetailPageKind {
        let visible_page = self.imp().detail_stack.visible_child();
        if visible_page.and_downcast_ref::<ArchivesPage>().is_some() {
            DetailPageKind::Archives
        } else if visible_page.and_downcast_ref::<SchedulePage>().is_some() {
            DetailPageKind::Schedule
        } else {
            DetailPageKind::Backup
        }
    }

    pub fn show_stack_page(&self, page_kind: DetailPageKind) {
        let imp = self.imp();
        match page_kind {
            DetailPageKind::Backup => imp.detail_stack.set_visible_child(&*imp.page_backup),
            DetailPageKind::Archives => imp.detail_stack.set_visible_child(&*imp.page_archives),
            DetailPageKind::Schedule => imp.detail_stack.set_visible_child(&*imp.page_schedule),
        }
    }

    pub fn show_pending_menu(&self, visible: bool) {
        self.imp().pending_menu.set_visible(visible);
    }

    pub fn backup_page(&self) -> &BackupPage {
        &self.imp().page_backup
    }

    pub fn archives_page(&self) -> &ArchivesPage {
        &self.imp().page_archives
    }

    pub fn schedule_page(&self) -> &SchedulePage {
        &self.imp().page_schedule
    }
}

impl HasAppWindow for DetailPage {
    fn app_window(&self) -> super::AppWindow {
        self.root()
            .and_downcast()
            .expect("PkDetailPage must be inside PkAppWindow")
    }
}
