mod display;
mod events;
mod execution;
pub mod init;

pub use display::{refresh, refresh_disk_status, refresh_status};
pub use events::on_stop_backup_create;

use crate::schedule;
use crate::ui::prelude::*;

pub fn start_backup(id: ConfigId, due_cause: Option<schedule::DueCause>, guard: QuitGuard) {
    // We spawn a new task instead of waiting for backup completion here.
    //
    // This is necessary because we can start backups from many different sources, including dbus.
    // If we waited here we wouldn't be receiving any more dbus messages until this backup is finished.
    Handler::run(async move {
        execution::backup(
            BACKUP_CONFIG.load().try_get(&id)?.clone(),
            due_cause,
            &guard,
        )
        .await
    });
}

fn is_visible() -> bool {
    super::page_detail::is_visible(&main_ui().page_backup())
}

pub fn view_backup_conf(id: &ConfigId) {
    ACTIVE_BACKUP_ID.update(|active_id| *active_id = Some(id.clone()));

    main_ui()
        .detail_stack()
        .set_visible_child(&main_ui().page_backup());

    main_ui()
        .navigation_view()
        .push(&main_ui().navigation_page_detail());
}
