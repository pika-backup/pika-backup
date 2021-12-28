use crate::ui;
use adw::prelude::*;

pub fn back_to_overview(ui: &ui::builder::DialogAddConfig) {
    ui.location_url().set_text("");

    ui.password().set_text("");
    ui.password_confirm().set_text("");

    ui.leaflet().set_visible_child(&ui.page_overview());
}
