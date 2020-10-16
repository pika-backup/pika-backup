use chrono::{Datelike, Utc};
use gtk::prelude::*;

use crate::ui::globals::*;
use crate::{ui, APPLICATION_NAME};

pub fn show() {
    let dialog = ui::builder::DialogAbout::new().dialog();
    dialog.set_transient_for(Some(&main_ui().window()));

    dialog.set_logo(None);
    dialog.set_program_name(APPLICATION_NAME);

    dialog.set_version(Some(env!("CARGO_PKG_VERSION")));
    dialog.set_comments(Some(env!("CARGO_PKG_DESCRIPTION")));
    dialog.set_website(Some(env!("CARGO_PKG_HOMEPAGE")));
    dialog.set_authors(&env!("CARGO_PKG_AUTHORS").split(',').collect::<Vec<&str>>());
    dialog.set_copyright(Some(&format!(
        "Copyright © 2018–{} Sophie Herold",
        Utc::now().year()
    )));

    dialog.show_all();
}
