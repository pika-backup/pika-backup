use crate::ui::prelude::*;
use adw::prelude::*;

use super::cache;
use super::display;
use super::events;

pub fn init() {
    main_ui().detail_stack().connect_visible_child_notify(|_| {
        if super::is_visible() {
            Handler::run(display::show());
        }
    });

    main_ui()
        .archives_prefix_edit()
        .connect_activated(|_| Handler::run(events::edit_prefix()));

    main_ui()
        .archives_cleanup()
        .connect_activated(|_| Handler::run(events::cleanup()));

    main_ui().refresh_archives().connect_clicked(|_| {
        Handler::run(async move {
            let config = BACKUP_CONFIG.load().active()?.clone();
            cache::refresh_archives(config, None).await
        });
    });

    main_ui().archives_eject_button().connect_clicked(|_| {
        Handler::run(events::eject_button_clicked());
    });

    // spinner performance

    main_ui()
        .archives_reloading_spinner()
        .connect_map(|s| s.start());
    main_ui()
        .archives_reloading_spinner()
        .connect_unmap(|s| s.stop());
}
