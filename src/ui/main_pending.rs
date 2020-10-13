use gtk::prelude::*;
use std::cell::Cell;

use crate::ui::globals::*;
thread_local!(
    static LAST_PAGE: Cell<Option<gtk::Widget>> = Default::default();
    static LAST_SHOW: Cell<Option<bool>> = Default::default();
);

pub fn init() {
    main_ui().page_pending_spinner().connect_map(|s| s.start());
    main_ui().page_pending_spinner().connect_unmap(|s| s.stop());
}

pub fn show(msg: &str) {
    LAST_PAGE.with(|last_page| last_page.set(main_ui().content_stack().get_visible_child()));
    LAST_SHOW.with(|last_show| last_show.set(Some(main_ui().leaflet_left().get_visible())));
    main_ui()
        .content_stack()
        .set_visible_child(&main_ui().page_pending());
    main_ui().add_pending_label().set_text(msg);
    main_ui().leaflet_left().hide();
}

pub fn back() {
    LAST_PAGE.with(|last_page| {
        if let Some(page) = last_page.take() {
            main_ui().content_stack().set_visible_child(&page)
        }
    });

    LAST_SHOW.with(|last_show| {
        if let Some(show) = last_show.take() {
            main_ui().leaflet_left().set_visible(show)
        }
    })
}
