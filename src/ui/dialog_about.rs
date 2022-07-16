use gtk::prelude::*;

use crate::ui;
use crate::ui::prelude::*;

pub fn show() {
    let dialog = ui::builder::DialogAbout::new().dialog();
    dialog.set_transient_for(Some(&main_ui().window()));

    dialog.set_application_icon(&crate::app_id());

    /*
    Translators: "Pika" in this app's name refers to a small mammal. If you transliterate "Pika," \
    please make sure that the transliteration does not coincide with a different meaning. If \
    fitting, translations of "Pika" are welcome too.

    <https://en.wikipedia.org/wiki/Pika>
    */
    dialog.set_application_name(&gettext("Pika Backup"));

    dialog.set_version(env!("CARGO_PKG_VERSION"));
    dialog.set_website(env!("CARGO_PKG_HOMEPAGE"));
    dialog.set_issue_url("https://gitlab.gnome.org/World/pika-backup/-/issues");

    dialog.set_developer_name(&gettext("Sophie Herold"));
    dialog.set_developers(&[&gettext("Sophie Herold <sophieherold@gnome.org>")]);
    dialog.set_copyright(&gettext("Copyright © 2018–2022 Sophie Herold et al."));
    dialog.set_translator_credits(&gettext("translator-credits"));
    dialog.add_credit_section(
        // Translators: This is an inside joke
        Some(&gettext("Court Witch")),
        &[&gettext("Fina Wilke")],
    );
    dialog.set_artists(&["Jakub Steiner", "Tobias Bernard"]);
    dialog.set_comments(&(
        String::from("<span line_height='1.6' weight='bold'>") +
        &gettext("Back End") +
        "</span>\n" +
        &gettext("Pika Backup uses BorgBackup. This app wouldn’t exist without Borg. Consider <a href='https://www.borgbackup.org/support/fund.html'>supporting BorgBackup</a>.") +
        "\n\n<span line_height='1.6' weight='bold'>" +
        &gettext("The Name") +
        &gettext("</span>\n") +
        &gettext("The name “Pika Backup” derives from the American pika. Pikas are small mammals belonging to the lagomorphs. The American pika is known for caching food in skillfully constructed haypiles. Techniques used by American pikas for building haypiles include: storing piles below overhanging rocks, using certain plants which inhibit bacterial growth as preservative and preferring plants with high nutrition value for haying. Last but not least, pikas do not only create very sophisticated haypiles but are quick in collecting vegetation as well.") +
        "\n\n" +
        &gettext("Frequent rumors that this software’s name is related to a monster with electrical abilities are unfounded.")
    ));

    dialog.show();
}
