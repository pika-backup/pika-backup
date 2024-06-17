use crate::ui;
use crate::ui::prelude::*;

pub fn window() -> adw::AboutDialog {
    let dialog = adw::AboutDialog::builder()
        /*
        Translators: "Pika" in this app's name refers to a small mammal. If you transliterate "Pika," \
        please make sure that the transliteration does not coincide with a different meaning. If \
        fitting, translations of "Pika" are welcome too.
        <https://en.wikipedia.org/wiki/Pika>
        */
        .application_name(gettext("Pika Backup"))
        .application_icon(crate::APP_ID)
        .version(env!("CARGO_PKG_VERSION"))
        .website(env!("CARGO_PKG_HOMEPAGE"))
        .issue_url("https://gitlab.gnome.org/World/pika-backup/-/issues")
        // Translators: Do not translate, only transliterate if necessary
        .developer_name(gettext("Small Mammal Collective"))
        .developers([
            gettext("Sophie Herold <sophieherold@gnome.org>").as_str(),
            gettext("Fina Wilke <finaw@gnome.org>").as_str(),
        ])
        .copyright(gettext("Copyright © 2018–2023 Sophie Herold et al."))
        .translator_credits(gettext("translator-credits"))
        .artists([
            gettext("Jakub Steiner").as_str(),
            gettext("Tobias Bernard").as_str(),
        ])
        .comments(format!("<span line_height='1.6' weight='bold'>{}</span>\n{}\n\n<span line_height='1.6' weight='bold'>{}</span>\n{}\n\n{}",
            gettext("Back End"),
            gettext("Pika Backup uses BorgBackup. This app wouldn’t exist without Borg. Consider <a href='https://www.borgbackup.org/support/fund.html'>supporting BorgBackup</a>."),
            gettext("The Name"),
            gettext("The name “Pika Backup” derives from the American pika. Pikas are small mammals belonging to the lagomorphs. The American pika is known for caching food in skillfully constructed haypiles. Techniques used by American pikas for building haypiles include: storing piles below overhanging rocks, using certain plants which inhibit bacterial growth as preservative and preferring plants with high nutrition value for haying. Last but not least, pikas do not only create very sophisticated haypiles but are quick in collecting vegetation as well."),
            gettext("Frequent rumors that this software’s name is related to a monster with electrical abilities are unfounded.")
        ))
        .debug_info(debug_info())
        .build();

    dialog.add_link(
        &gettext("Support us on Open Collective"),
        "https://opencollective.com/pika-backup",
    );
    dialog.add_link(
        &gettext("Support us on GitHub Sponsors"),
        "https://github.com/sponsors/pika-backup/",
    );

    dialog
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
            "- Sandboxed: {} ({})",
            *crate::globals::APP_IS_SANDBOXED,
            std::env::var("container").unwrap_or_default()
        ),
        format!(
            "- gtk: {}.{}.{}",
            gtk::major_version(),
            gtk::minor_version(),
            gtk::micro_version()
        ),
        format!(
            "- libadwaita: {}.{}.{}",
            adw::major_version(),
            adw::minor_version(),
            adw::micro_version()
        ),
        format!(
            "- BorgBackup: {}",
            crate::ui::BORG_VERSION
                .get()
                .map_or("Unknown", String::as_str)
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
