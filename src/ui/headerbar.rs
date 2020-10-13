use gtk::prelude::*;
use libhandy::prelude::*;

use crate::ui::globals::*;

pub fn init() {
    update();

    main_ui()
        .content_leaflet()
        .connect_property_folded_notify(leaflet_folded);
    main_ui()
        .content_stack()
        .connect_property_visible_child_notify(content_stack_changed);

    main_ui().show_overview().connect_clicked(show_overview);
}

pub fn update() {
    leaflet_folded(&main_ui().content_leaflet());
    main_ui()
        .remove_backup()
        .set_sensitive(!ACTIVE_BACKUP_ID.load().is_none());
}

fn content_stack_changed(stack: &gtk::Stack) {
    let sensitive =
        stack.get_visible_child() == Some(main_ui().page_main().upcast::<gtk::Widget>());

    main_ui()
        .view_switcher_title()
        .set_view_switcher_enabled(sensitive);
    main_ui().view_switcher_bottom().set_visible(sensitive);
}

fn leaflet_folded(leaflet: &libhandy::Leaflet) {
    if SETTINGS.load().backups.len() > 1 {
        if leaflet.get_folded() {
            main_ui()
                .headerbar_right_buttons()
                .set_visible_child(&main_ui().show_overview());
        } else {
            main_ui()
                .headerbar_right_buttons()
                .set_visible_child(&main_ui().headerbar_nothing());
        }
    } else {
        main_ui()
            .headerbar_right_buttons()
            .set_visible_child(&main_ui().add_backup_right());
    }
}

fn show_overview(_: &gtk::Button) {
    main_ui()
        .content_leaflet()
        .set_visible_child(&main_ui().leaflet_left());
}
