mod display;
mod events;
mod execution;
pub mod init;

pub use display::{refresh, refresh_status};

use crate::ui::prelude::*;

pub fn activate_action_backup(id: ConfigId) {
    Handler::run(async move {
        execution::start_backup(BACKUP_CONFIG.load().get_result(&id)?.clone()).await
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
        .leaflet()
        .set_visible_child(&main_ui().page_detail());
}
