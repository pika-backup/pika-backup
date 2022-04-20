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
    end: chrono::NaiveDateTime,
}

fn extract_archive_params(archive: borg::ListArchive) -> ArchiveParams {
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
        end: archive.end,
    }
}

pub fn transfer_selection(
    ui: &DialogSetup,
    config_id: config::ConfigId,
    archives: Vec<borg::ListArchive>,
) {
    let archive_params: Vec<_> = archives.into_iter().map(extract_archive_params).collect();

    let valid_prefixes: Vec<_> = archive_params
        .iter()
        .map(|x| &x.prefix)
        .duplicates()
        .collect();

    let options = archive_params
        .iter()
        .filter(|x| valid_prefixes.contains(&&x.prefix))
        .unique_by(|x| (&x.prefix, &x.parsed, &x.hostname, &x.username));

    for suggestion in options.take(10) {
        let row = ui::builder::DialogSetupTransferOption::new();

        row.hostname().set_label(&suggestion.hostname);
        row.username().set_label(&suggestion.username);
        row.prefix().set_label(
            &suggestion
                .prefix
                .as_ref()
                .map(|x| x.to_string())
                .unwrap_or_else(|| gettext("None")),
        );

        for include in suggestion.parsed.include.iter() {
            let tag = ui::widget::LocationTag::from_path(include.clone());
            row.include().add_child(&tag.build());
        }

        for exclude in suggestion.parsed.exclude.iter() {
            let tag = ui::widget::LocationTag::from_pattern(exclude.clone());
            row.exclude().add_child(&tag.build());
        }

        row.transfer().connect_activated(
            clone!(@weak ui, @strong suggestion, @strong config_id => move |_|
            Handler::handle(insert_transfer(ui, &suggestion, &config_id))
            ),
        );

        ui.transfer_suggestions().prepend(&row.widget());
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
            conf.set_archive_prefix(prefix.clone(), BACKUP_CONFIG.load().iter()).err_to_msg(gettext("Invalid Archive Prefix"))?;
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
