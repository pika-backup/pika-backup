use adw::prelude::*;

use crate::ui;

use crate::ui::prelude::*;

use super::events;

pub fn init() {
    main_ui()
        .backup_run()
        .connect_clicked(|_| Handler::run(events::on_backup_run()));

    // Backup details
    main_ui()
        .detail_status_row()
        .connect_activated(|_| ui::dialog_info::show());

    main_ui()
        .detail_repo_row()
        .add_prefix(&main_ui().detail_repo_icon());

    main_ui()
        .detail_repo_row()
        .connect_activated(|_| Handler::run(ui::dialog_storage::show()));

    main_ui()
        .navigation_view()
        .connect_visible_page_notify(|navigation_view| {
            if navigation_view
                .visible_page()
                .is_some_and(|page| page == main_ui().navigation_page_detail())
            {
                events::on_stack_changed()
            }
        });

    main_ui()
        .detail_stack()
        .connect_visible_child_notify(|_| events::on_stack_changed());

    main_ui()
        .add_include()
        .connect_clicked(|_| Handler::run(events::add_include()));
    main_ui()
        .add_exclude()
        .connect_clicked(|_| Handler::run(events::add_exclude()));

    main_ui()
        .stop_backup_create()
        .connect_clicked(|_| Handler::run(events::on_stop_backup_create()));

    main_ui()
        .backup_disk_eject_button()
        .connect_clicked(|_| Handler::run(events::on_backup_disk_eject()));
}
