use crate::ui::prelude::*;
use adw::prelude::*;

use itertools::Itertools;
use std::collections::BTreeSet;
use std::fmt::Write;

use crate::borg;
use crate::config;
use crate::ui;
use ui::builder::DialogSetup;

#[derive(Clone, Debug)]
struct ArchiveParams {
    prefix: Option<config::ArchivePrefix>,
    parsed: borg::invert_command::Parsed,
    hostname: String,
    username: String,
    end: chrono::NaiveDateTime,
    stats: borg::json::Stats,
}

fn extract_archive_params(archive: borg::ListArchive) -> ArchiveParams {
    let prefix = archive
        .name
        .as_str()
        .split_once('-')
        .map(|x| config::ArchivePrefix(x.0.to_string() + "-"));
    let stats = borg::json::Stats::transfer_history_mock(&archive);
    let parsed = borg::invert_command::parse(archive.command_line);

    ArchiveParams {
        prefix,
        parsed,
        hostname: archive.hostname,
        username: archive.username,
        end: archive.end,
        stats,
    }
}

pub fn transfer_selection(
    ui: &DialogSetup,
    config_id: config::ConfigId,
    archives: Vec<borg::ListArchive>,
) {
    let archive_params: Vec<_> = archives
        .into_iter()
        .map(extract_archive_params)
        .rev()
        .collect();

    let valid_prefixes: Vec<_> = archive_params
        .iter()
        .map(|x| &x.prefix)
        .duplicates()
        .collect();

    let mut options = archive_params
        .iter()
        .filter(|x| valid_prefixes.contains(&&x.prefix))
        .unique_by(|x| (&x.prefix, &x.parsed, &x.hostname, &x.username))
        .peekable();

    if options.peek().is_none() {
        ui.dialog().close();
    } else {
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
                let tag = ui::widget::LocationTag::from_exclude(exclude.clone().into_relative());
                row.exclude().add_child(&tag.build());
            }

            row.transfer().connect_activated(
                clone!(@weak ui, @strong suggestion, @strong config_id => move |_|
                Handler::handle(insert_transfer(ui, &suggestion, &config_id))
                ),
            );

            ui.transfer_suggestions().append(&row.widget());
        }

        ui.page_transfer()
            .set_visible_child(&ui.page_transfer_select());
    }
}

fn insert_transfer(
    ui: DialogSetup,
    archive_params: &ArchiveParams,
    config_id: &ConfigId,
) -> Result<()> {
    BACKUP_CONFIG.update_result(enclose!((archive_params, config_id) move |config| {
        let mut conf = config.get_result_mut(&config_id)?;

        conf.include = archive_params.parsed.include.clone();
        conf.exclude = BTreeSet::from_iter( archive_params.parsed.exclude.clone().into_iter().map(|x| x.into_relative()));

        Ok(())
    }))?;

    let entry = config::history::RunInfo {
        end: archive_params
            .end
            .and_local_timezone(chrono::Local)
            .unwrap(),
        outcome: borg::Outcome::Completed {
            stats: archive_params.stats.clone(),
        },
        messages: Default::default(),
        include: archive_params.parsed.include.clone(),
        exclude: archive_params.parsed.exclude.clone(),
    };

    BACKUP_HISTORY.update(
        enclose!((config_id) move |histories| histories.insert(config_id.clone(), entry.clone())),
    );

    // Create fake history entry for duration estimate to be good for first run

    crate::ui::write_config()?;
    ui::page_backup::refresh()?;

    let configs = BACKUP_CONFIG.load();
    let config = configs.get_result(config_id)?;

    if let Some(prefix) = &archive_params.prefix {
        ui.prefix().set_text(&prefix.to_string());
    } else {
        ui.prefix().set_text(&config.archive_prefix.to_string());
    }
    ui.prefix().grab_focus();
    ui.dialog().set_default_widget(Some(&ui.prefix_submit()));

    ui.prefix_submit().connect_clicked(clone!(@weak ui, @strong config_id => move |_|
        Handler::new().error_transient_for(ui.dialog()).handle_sync(set_prefix(&ui, config_id.clone()))));

    ui.page_transfer()
        .set_visible_child(&ui.page_transfer_prefix());

    Ok(())
}

pub fn set_prefix(ui: &DialogSetup, config_id: ConfigId) -> Result<()> {
    let prefix = ui.prefix().text();

    BACKUP_CONFIG.update_result(enclose!((prefix, config_id) move |config| {
        config
            .get_result_mut(&config_id)?
            .set_archive_prefix(
                config::ArchivePrefix::new(&prefix),
                BACKUP_CONFIG.load().iter(),
            )
            .err_to_msg(gettext("Invalid Archive Prefix"))
    }))?;

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
            let _ = write!(label1, " – {}", &glib::format_size(df.size));

            label2.push_str(" – ");
            label2.push_str(&gettextf("Free space: {}", &[&glib::format_size(df.avail)]));
        }

        if let Some(repo_path) = repo {
            row.set_widget_name(&gio::File::for_path(repo_path).uri());
            if let Ok(suffix) = repo_path.strip_prefix(mount_path) {
                if !suffix.to_string_lossy().is_empty() {
                    let _ = write!(label1, " / {}", suffix.display());
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
