use gtk::prelude::*;

use crate::ui;
use crate::ui::prelude::*;

pub fn show() {
    let dialog = ui::builder::DialogAbout::new().dialog();
    dialog.set_transient_for(Some(&main_ui().window()));

    dialog.set_application_icon(crate::APP_ID);

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
    dialog.add_link(
        &gettext("Support us on Open Collective"),
        "https://opencollective.com/pika-backup",
    );
    dialog.add_link(
        &gettext("Support us on GitHub Sponsors"),
        "https://github.com/sponsors/pika-backup/",
    );

    dialog.set_developer_name(&gettext("Small Mammal Collective"));
    dialog.set_developers(&[
        &gettext("Sophie Herold <sophieherold@gnome.org>"),
        &gettext("Fina Wilke <finaw@gnome.org>"),
    ]);
    dialog.set_copyright(&gettext("Copyright © 2018–2023 Sophie Herold et al."));
    dialog.set_translator_credits(&gettext("translator-credits"));
    dialog.set_artists(&["Jakub Steiner", "Tobias Bernard"]);
    dialog.set_comments(&(
        String::from("<span line_height='1.6' weight='bold'>") +
        &gettext("Back End") +
        "</span>\n" +
        &gettext("Pika Backup uses BorgBackup. This app wouldn’t exist without Borg. Consider <a href='https://www.borgbackup.org/support/fund.html'>supporting BorgBackup</a>.") +
        "\n\n<span line_height='1.6' weight='bold'>" +
        &gettext("The Name") +
        "</span>\n" +
        &gettext("The name “Pika Backup” derives from the American pika. Pikas are small mammals belonging to the lagomorphs. The American pika is known for caching food in skillfully constructed haypiles. Techniques used by American pikas for building haypiles include: storing piles below overhanging rocks, using certain plants which inhibit bacterial growth as preservative and preferring plants with high nutrition value for haying. Last but not least, pikas do not only create very sophisticated haypiles but are quick in collecting vegetation as well.") +
        "\n\n" +
        &gettext("Frequent rumors that this software’s name is related to a monster with electrical abilities are unfounded.")
    ));

    dialog.set_debug_info(&debug_info());

    dialog.show();
}

fn etc() -> std::path::PathBuf {
    if *crate::globals::APP_IS_SANDBOXED {
        std::path::PathBuf::from("/run/host/etc")
    } else {
        std::path::PathBuf::from("/etc")
    }
}

fn os_release() -> String {
    std::fs::read_to_string(etc().join("os-release")).unwrap_or_default()
}

fn user_autostart() -> String {
    std::fs::read_to_string(
        crate::utils::host::user_config_dir().join(format!("autostart/{}.desktop", crate::APP_ID)),
    )
    .unwrap_or_default()
}

fn global_autostart() -> String {
    std::fs::read_to_string(etc().join(format!("xdg/autostart/{}.desktop", crate::APP_ID)))
        .unwrap_or_default()
}

fn debug_info() -> String {
    [
        format!("- Version: {}", env!("CARGO_PKG_VERSION")),
        format!(
            "- Commit: {}",
            option_env!("GIT_DESCRIBE").unwrap_or("not set")
        ),
        format!("- App ID: {}", crate::APP_ID),
        format!(
            "- Sandboxed: {} {}",
            *crate::globals::APP_IS_SANDBOXED,
            std::env::var("container").unwrap_or_default()
        ),
        format!("\n##### OS Information\n```\n{}\n```", os_release()),
        format!(
            "\n##### Flatpak Information\n```\n{:#?}\n```",
            ui::utils::flatpak_info::get()
        ),
        format!("\n##### User Autostart\n```\n{}\n```", user_autostart()),
        format!("\n##### Global Autostart\n```\n{}\n```", global_autostart()),
    ]
    .join("\n")
}
