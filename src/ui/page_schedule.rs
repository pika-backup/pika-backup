use crate::ui::prelude::*;
use adw::prelude::*;

mod event;
pub mod frequency;
pub mod init;
pub mod prune_preset;
pub mod status;
pub mod weekday;

pub fn dbus_show(id: ConfigId) {
    view(&id);
    adw_app().activate();
}

pub fn view(id: &ConfigId) {
    ACTIVE_BACKUP_ID.update(|active_id| *active_id = Some(id.clone()));

    main_ui()
        .navigation_view()
        .push(&main_ui().navigation_page_detail());
    main_ui()
        .detail_stack()
        .set_visible_child(&main_ui().page_schedule());
}

pub fn is_visible() -> bool {
    super::page_detail::is_visible(&main_ui().page_schedule())
}

pub fn refresh_status() {
    if is_visible() {
        if let Ok(config) = BACKUP_CONFIG.load().active().cloned() {
            glib::MainContext::default()
                .spawn_local(async move { event::update_status(&config).await });
        }
    }
}
