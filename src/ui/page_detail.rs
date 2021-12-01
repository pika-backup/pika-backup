use crate::ui::prelude::*;
use gtk::prelude::*;

pub fn is_visible(page: &gtk::ScrolledWindow) -> bool {
    main_ui().leaflet().visible_child() == Some(main_ui().page_detail().upcast::<gtk::Widget>())
        && main_ui().detail_stack().visible_child() == Some(page.clone().upcast::<gtk::Widget>())
}
