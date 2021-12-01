use crate::ui::prelude::*;
use gtk::prelude::*;

pub fn init() {
    main_ui()
        .leaflet()
        .connect_child_transition_running_notify(on_transition);
}

pub fn is_visible(page: &gtk::ScrolledWindow) -> bool {
    is_leaflet_visible()
        && main_ui().detail_stack().visible_child() == Some(page.clone().upcast::<gtk::Widget>())
}

pub fn is_leaflet_visible() -> bool {
    main_ui().leaflet().visible_child() == Some(main_ui().page_detail().upcast::<gtk::Widget>())
}

pub fn on_transition(stack: &adw::Leaflet) {
    if !stack.is_child_transition_running() && !is_leaflet_visible() {
        for scrollable in &[
            main_ui().page_backup(),
            main_ui().page_archives(),
            main_ui().page_schedule(),
        ] {
            scrollable
                .vadjustment()
                .unwrap()
                .set_value(scrollable.vadjustment().unwrap().lower());
        }
    }
}
