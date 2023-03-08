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
    main_ui().leaflet().set_visible_child(&main_ui().overview());
}

fn main_stack_changed(stack: &adw::ViewStack) {
    let is_detail_view =
        stack.visible_child() == Some(main_ui().page_detail().upcast::<gtk::Widget>());

    // shown in overview
    main_ui().add_backup().set_visible(!is_detail_view);
    main_ui().primary_menu_button().set_visible(!is_detail_view);
}
