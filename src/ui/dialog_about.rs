use gtk::prelude::*;

use crate::ui;
use crate::ui::prelude::*;

pub fn show() {
    let dialog = ui::builder::DialogAbout::new().dialog();
    dialog.set_transient_for(Some(&main_ui().window()));

    dialog.set_logo(None);

    /*
    Translators: "Pika" in this app's name refers to a small mammal. If you transliterate "Pika," \
    please make sure that the transliteration does not coincide with a different meaning. If \
    fitting, translations of "Pika" are welcome too.

    <https://en.wikipedia.org/wiki/Pika>
    */
    dialog.set_program_name(&gettext("Pika Backup"));

    dialog.set_version(Some(env!("CARGO_PKG_VERSION")));
    dialog.set_comments(Some(env!("CARGO_PKG_DESCRIPTION")));
    dialog.set_website(Some(env!("CARGO_PKG_HOMEPAGE")));
    dialog.set_authors(&[&gettext("Sophie Herold <sophieherold@gnome.org>")]);
    dialog.set_copyright(Some(&gettext("Copyright © 2018–2021 Sophie Herold et al.")));
    dialog.set_translator_credits(Some(&gettext("translator-credits")));
    dialog.add_credit_section(&gettext("Court Witch"), &[&gettext("Fina Wilke")]);
    dialog.set_artists(&["Jakub Steiner"]);

    dialog.show_all();
}
