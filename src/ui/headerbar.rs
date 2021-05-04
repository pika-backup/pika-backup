use gtk::prelude::*;

use crate::ui::globals::*;

pub fn init() {
    main_ui()
        .main_stack()
        .connect_visible_child_notify(main_stack_changed);

    main_ui().back_button().connect_clicked(on_back);

    main_ui().pending_menu_spinner().connect_map(|s| s.start());
    main_ui().pending_menu_spinner().connect_unmap(|s| s.stop());

    main_stack_changed(&main_ui().main_stack());
}

fn on_back(_button: &gtk::Button) {
    main_ui()
        .main_stack()
        .set_visible_child(&main_ui().page_overview());
}

fn main_stack_changed(stack: &gtk::Stack) {
    let is_detail_view =
        stack.visible_child() == Some(main_ui().page_detail().upcast::<gtk::Widget>());

    // shown in detail view
    main_ui()
        .view_switcher_title()
        .set_view_switcher_enabled(is_detail_view);
    main_ui().view_switcher_bottom().set_visible(is_detail_view);
    main_ui().back_button().set_visible(is_detail_view);
    main_ui()
        .secondary_menu_button()
        .set_visible(is_detail_view);

    // shown in overview
    main_ui().add_backup().set_visible(!is_detail_view);
    main_ui().primary_menu_button().set_visible(!is_detail_view);
}
