use adw::prelude::*;

use crate::ui::builder::DialogShortcuts;
use crate::ui::prelude::*;

pub fn show() {
    let ui = DialogShortcuts::new();

    ui.dialog().set_transient_for(Some(&main_ui().window()));

    ui.dialog().present();
}
