use gtk::prelude::*;
use std::cell::Cell;

use crate::ui::globals::*;
thread_local!(
    static LAST_PAGE: Cell<Option<gtk::Widget>> = Default::default();
);

pub fn show(msg: &str) {
    LAST_PAGE.with(|last_page| last_page.set(main_ui().main_stack().get_visible_child()));
    main_ui()
        .main_stack()
        .set_visible_child(&main_ui().page_pending());
    main_ui().add_pending_label().set_text(msg);
}

pub fn back() {
    LAST_PAGE.with(|last_page| {
        if let Some(page) = last_page.take() {
            main_ui().main_stack().set_visible_child(&page)
        }
    })
}
