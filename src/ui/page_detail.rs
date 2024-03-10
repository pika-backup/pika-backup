use crate::ui::prelude::*;
use adw::prelude::*;

pub fn init() {
    main_ui().navigation_view().connect_pushed(on_pushed);
}

pub fn is_visible(page: &impl IsA<gtk::Widget>) -> bool {
    is_navigation_page_visible()
        && main_ui().detail_stack().visible_child() == Some(page.clone().upcast::<gtk::Widget>())
}

pub fn is_navigation_page_visible() -> bool {
    main_ui().navigation_view().visible_page() == Some(main_ui().navigation_page_detail())
}

pub fn on_pushed(_navigation_view: &adw::NavigationView) {
    if is_navigation_page_visible() {
        for page in &[
            main_ui().page_backup().upcast_ref::<adw::PreferencesPage>(),
            &main_ui().page_archives().upcast_ref(),
            &main_ui().page_schedule().upcast_ref(),
        ] {
            page.scroll_to_top();
        }
    }
}
