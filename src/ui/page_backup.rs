mod display;
mod events;
mod execution;
pub mod init;

pub use display::{refresh, refresh_disk_status, refresh_status};
pub use events::on_stop_backup_create;

use crate::schedule;
use crate::ui::prelude::*;

pub fn activate_action_backup(id: ConfigId) {
    Handler::run(async move {
        execution::start_backup(BACKUP_CONFIG.load().try_get(&id)?.clone(), None).await
    });
}

pub async fn dbus_start_backup(id: ConfigId, due_cause: Option<schedule::DueCause>) -> Result<()> {
    execution::start_backup(BACKUP_CONFIG.load().try_get(&id)?.clone(), due_cause).await
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
