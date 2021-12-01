use crate::ui::prelude::*;

mod event;
pub mod frequency;
pub mod init;
pub mod status;
pub mod weekday;

pub fn view(id: &ConfigId) {
    ACTIVE_BACKUP_ID.update(|active_id| *active_id = Some(id.clone()));

    main_ui()
        .leaflet()
        .set_visible_child(&main_ui().page_detail());
    main_ui()
        .detail_stack()
        .set_visible_child(&main_ui().page_schedule());
}

pub fn is_visible() -> bool {
    super::page_detail::is_visible(&main_ui().page_schedule())
}
