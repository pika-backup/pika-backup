use gtk::prelude::*;

use crate::ui::backup_status;
use crate::ui::globals::*;
pub fn init() {
    main_ui()
        .detail_running_backup_info()
        .connect_delete_event(|x, _| WidgetExtManual::hide_on_delete(x));

    glib::timeout_add_local(250, || {
        refresh_status();
        Continue(true)
    });
}

fn is_visible() -> bool {
    main_ui().detail_running_backup_info().get_visible()
}

fn refresh_status() {
    if is_visible() {
        if let Some(id) = ACTIVE_BACKUP_ID.load().as_ref().as_ref() {
            refresh_status_display(&backup_status::Display::new_from_id(id));
        }
    }
}

fn refresh_status_display(status: &backup_status::Display) {
    main_ui().detail_info_status().set_text(&status.title);

    if let Some(progress) = status.progress {
        main_ui().detail_info_progress().set_fraction(progress);
        main_ui().detail_info_progress().show();
    } else {
        main_ui().detail_info_progress().hide();
    }

    if let Some(ref subtitle) = status.subtitle {
        main_ui().detail_info_substatus().set_text(&subtitle);
        main_ui().detail_info_substatus().show();
    } else {
        main_ui().detail_info_substatus().hide();
    }
}
