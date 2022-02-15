use crate::ui::prelude::*;
use adw::prelude::*;

use itertools::Itertools;

use crate::borg;
use crate::config;
use crate::ui;
use ui::builder::DialogSetup;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct ArchiveParams {
    prefix: Option<config::ArchivePrefix>,
    parsed: borg::invert_command::Parsed,
    hostname: String,
    username: String,
}

fn extract_archive_params(archive: borg::InfoArchive) -> ArchiveParams {
    let prefix = archive
        .name
        .as_str()
        .split_once('-')
        .map(|x| config::ArchivePrefix(x.0.to_string() + "-"));
    let parsed = borg::invert_command::parse(archive.command_line);

    ArchiveParams {
        prefix,
        parsed,
        hostname: archive.hostname,
        username: archive.username,
    }
}

pub fn transfer_selection(
    ui: &DialogSetup,
    config_id: config::ConfigId,
    archives: Vec<borg::InfoArchive>,
) {
    let archive_params: Vec<_> = archives
        .into_iter()
        .map(extract_archive_params)
        .unique()
        .collect();

    for suggestion in archive_params {
        let row = adw::ActionRow::builder()
            .activatable(true)
            .title(&format!("{}: {}", suggestion.hostname, suggestion.username))
            .build();

        if let Some(prefix) = &suggestion.prefix {
            row.set_subtitle(&glib::markup_escape_text(&prefix.to_string()));
        }

        let box_ = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        box_.add_css_class("overview-includes");
        row.add_suffix(&box_);

        row.connect_activated(
            clone!(@weak ui, @strong suggestion, @strong config_id => move |_|
            Handler::handle(insert_transfer(ui, &suggestion, &config_id))
            ),
        );

        for include in suggestion.parsed.include {
            let label = gtk::Label::new(Some(&include.to_string_lossy()));
            let include_box = gtk::Box::new(gtk::Orientation::Horizontal, 3);
            include_box.append(&label);
            include_box.set_valign(gtk::Align::Center);
            box_.append(&include_box);
        }

        ui.transfer_suggestions().append(&row);
    }

    ui.page_transfer()
        .set_visible_child(&ui.page_transfer_select());
}

fn insert_transfer(
    ui: DialogSetup,
    archive_params: &ArchiveParams,
    config_id: &ConfigId,
) -> Result<()> {
    BACKUP_CONFIG.update_result(enclose!((archive_params, config_id) move |config| {
        let mut conf = config.get_result_mut(&config_id)?;

        conf.include = archive_params.parsed.include.clone();
        conf.exclude = archive_params.parsed.exclude.clone();

        if let Some(prefix) = &archive_params.prefix {
            conf.archive_prefix = prefix.clone();
        }

        Ok(())
    }))?;

    crate::ui::write_config()?;

    ui::page_backup::refresh()?;
    ui.dialog().close();

    Ok(())
}

pub fn show_init_remote(ui: &ui::builder::DialogSetup) {
    ui.location_stack().set_visible_child(&ui.location_remote());
    show_init(ui);
}

pub fn show_init_local(ui: &ui::builder::DialogSetup, path: Option<&std::path::Path>) {
    if let Some(path) = path {
        ui.init_path()
            .set_property("file", gio::File::for_path(path));
    }

    ui.location_stack().set_visible_child(&ui.location_local());
    show_init(ui);
}

fn show_init(ui: &ui::builder::DialogSetup) {
    ui.init_dir().set_text(&format!(
        "backup-{}-{}",
        glib::host_name(),
        glib::user_name().to_string_lossy()
    ));

    ui.password_quality().set_value(0.0);

    ui.password().set_text("");
    ui.password_confirm().set_text("");

    ui.leaflet().set_visible_child(&ui.page_detail());

    ui.encryption_box().show();

    ui.ask_password().set_text("");

    ui.button_stack().set_visible_child(&ui.init_button());
    ui.dialog().set_default_widget(Some(&ui.init_button()));
}

pub async fn add_mount<F: 'static + Fn()>(
    list: &gtk::ListBox,
    mount: &gio::Mount,
    repo: Option<&std::path::Path>,
    display_fn: F,
) {
    let row = ui::utils::new_action_row_with_gicon(Some(mount.icon().as_ref()));
    list.append(&row);

    row.set_widget_name(&mount.root().uri());
    row.connect_activated(move |_| display_fn());

    let mut label1 = mount.name().to_string();

    let mut label2: String = mount
        .drive()
        .as_ref()
        .map(gio::Drive::name)
        .map(Into::into)
        .unwrap_or_else(|| mount.root().uri().to_string());

    if let Some(mount_path) = mount.root().path() {
        if let Ok(df) = ui::utils::df::local(&mount_path).await {
            label1.push_str(&format!(" – {}", &glib::format_size(df.size)));

            label2.push_str(" – ");
            label2.push_str(&gettextf("Free space: {}", &[&glib::format_size(df.avail)]));
        }

        if let Some(repo_path) = repo {
            row.set_widget_name(&gio::File::for_path(repo_path).uri());
            if let Ok(suffix) = repo_path.strip_prefix(mount_path) {
                if !suffix.to_string_lossy().is_empty() {
                    label1.push_str(&format!(" / {}", suffix.to_string_lossy()));
                }
            }
        }
    }

    row.set_title(&glib::markup_escape_text(&label1));
    row.set_subtitle(&glib::markup_escape_text(&label2));
}

pub fn pending_check(ui: &DialogSetup) {
    ui.page_password()
        .set_visible_child(&ui.page_password_pending());
    ui.leaflet().set_visible_child(&ui.page_password());
}

pub fn ask_password(ui: &DialogSetup) {
    ui.page_password()
        .set_visible_child(&ui.page_password_input());
}
