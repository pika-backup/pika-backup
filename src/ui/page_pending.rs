use gtk::prelude::*;
use std::cell::Cell;

use crate::ui::globals::*;
thread_local!(
    static LAST_PAGE: Cell<Option<gtk::Widget>> = Default::default();
);

pub fn init() {
    main_ui().page_pending_spinner().connect_map(|s| s.start());
    main_ui().page_pending_spinner().connect_unmap(|s| s.stop());

    main_ui()
        .main_stack()
        .connect_visible_child_notify(on_main_stack_changed);
}

pub fn show(msg: &str) {
    if main_ui().main_stack().visible_child()
        != Some(main_ui().page_pending().upcast::<gtk::Widget>())
    {
        LAST_PAGE.with(|last_page| last_page.set(main_ui().main_stack().visible_child()));
    }
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
    });
}

fn on_main_stack_changed(stack: &gtk::Stack) {
    if stack.visible_child() == Some(main_ui().page_pending().upcast::<gtk::Widget>()) {
        main_ui().add_backup().hide();
    }
}
