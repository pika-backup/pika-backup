use chrono::prelude::*;
use gio::prelude::*;
use gtk::prelude::*;
use libhandy::prelude::*;

use crate::borg;
use crate::borg::Run;
use crate::shared;
use crate::shared::*;
use crate::ui;
use crate::ui::globals::*;
use crate::ui::prelude::*;
use crate::ui::utils::{self, WidgetEnh};

pub fn init() {
    main_ui().include_home_row().add(&main_ui().include_home());

    main_ui().backup_run().connect_clicked(|_| on_backup_run());

    main_ui()
        .detail_status_row()
        .add_prefix(&main_ui().status_icon());
    main_ui()
        .detail_status_row()
        .add(&main_ui().stop_backup_create());

    main_ui().detail_status_row().set_activatable(true);
    main_ui()
        .detail_status_row()
        .connect_activated(|_| main_ui().detail_running_backup_info().show_all());
    main_ui()
        .detail_running_backup_info()
        .connect_delete_event(|x, _| WidgetExtManual::hide_on_delete(x));

    main_ui()
        .detail_repo_row()
        .add_prefix(&main_ui().detail_repo_icon());

    main_ui().detail_repo_row().set_activatable(true);
    main_ui()
        .detail_repo_row()
        .connect_activated(|_| ui::dialog_storage::show());

    main_ui()
        .main_stack()
        .connect_property_transition_running_notify(on_transition);

    main_ui()
        .include_home()
        .connect_property_active_notify(|switch| {
            if switch.get_sensitive() {
                let change: bool = if switch.get_active() {
                    true
                } else {
                    ui::utils::dialog_yes_no(gettext(
                        "Are you sure you want to remove the home directory from this backup?",
                    ))
                };

                SETTINGS.update(|settings| {
                    if !change {
                        switch.set_active(!switch.get_active());
                    } else if switch.get_active() {
                        settings
                            .backups
                            .get_active_mut()
                            .unwrap()
                            .include
                            .insert(std::path::PathBuf::new());
                    } else {
                        settings
                            .backups
                            .get_active_mut()
                            .unwrap()
                            .include
                            .remove(&std::path::PathBuf::new());
                    }
                });

                if change {
                    super::write_config();
                    refresh();
                }
            }
        });

    main_ui().add_include().connect_clicked(|_| add_include());
    main_ui().add_exclude().connect_clicked(|_| add_exclude());

    main_ui()
        .stop_backup_create()
        .connect_clicked(|_| stop_backup_create());

    main_ui().status_icon_spinner().connect_map(|s| s.start());
    main_ui().status_icon_spinner().connect_unmap(|s| s.stop());

    glib::timeout_add_local(500, || {
        refresh_statusx();
        Continue(true)
    });
}

pub fn view_backup_conf(id: &str) {
    ACTIVE_BACKUP_ID.update(|active_id| *active_id = Some(id.to_string()));
    refresh();

    main_ui()
        .detail_stack()
        .set_visible_child(&main_ui().page_backup());
    main_ui()
        .main_stack()
        .set_visible_child(&main_ui().page_detail());
}

fn stop_backup_create() {
    if !ui::utils::dialog_yes_no(gettext("Are you sure you want abort the running backup?")) {
        return;
    }

    if let Some(communication) = BACKUP_COMMUNICATION.load().get_active() {
        communication.instruction.update(|inst| {
            *inst = borg::Instruction::Abort;
        });
    }
}

fn on_backup_run() {
    let config = SETTINGS.load().backups.get_active().unwrap().clone();
    let backup_id = ACTIVE_BACKUP_ID.get().unwrap();

    if ACTIVE_MOUNTS.load().contains(&backup_id) {
        debug!("Trying to run borg::create on a backup that is currently mounted.");

        let unmount = ui::utils::dialog_yes_no(gettext(
            "The backup repository is currently reserved for browsing files. \
             Do you want to disable browsing and start the Backup?",
        ));
        if unmount {
            trace!("User decided to unmount repo.");
            if !ui::utils::dialog_catch_err(
                borg::Borg::new(config.clone()).umount(),
                gettext("Failed to unmount repository."),
            ) {
                ACTIVE_MOUNTS.update(|mounts| {
                    mounts.remove(&backup_id);
                });
            }
        } else {
            trace!("User decided to abort backup.");
            return;
        }
    }

    ui::dialog_device_missing::main(config.clone(), move || run_backup(config.clone()));
}

pub fn run_backup(config: shared::BackupConfig) {
    //let backup_id = backup.id.clone();

    let communication: borg::Communication = Default::default();

    BACKUP_COMMUNICATION.update(|x| {
        x.insert(config.id.clone(), communication.clone());
    });
    refresh_status(&communication);

    ui::utils::Async::borg(
        "borg::create",
        borg::Borg::new(config.clone()),
        move |borg| borg.create(communication),
        move |result| {
            BACKUP_COMMUNICATION.update(|c| {
                c.remove(&config.id);
            });
            let user_aborted = matches!(result, Err(shared::BorgErr::UserAborted));
            let result_string_err = result.map_err(|err| format!("{}", err));
            let run_info = Some(shared::RunInfo::new(result_string_err.clone()));
            refresh_offline(&run_info);
            SETTINGS.update(|settings| {
                settings.backups.get_mut(&config.id).unwrap().last_run = run_info.clone()
            });
            ui::write_config();

            if !user_aborted {
                ui::utils::dialog_catch_errb(&result_string_err, gettext("Backup failed"));
                ui::page_archives::refresh_archives_cache(config.clone());
            }
        },
    );
}

pub fn add_list_row(list: &gtk::ListBox, file: &std::path::Path) -> gtk::Button {
    let row = libhandy::ActionRow::new();
    list.add(&row);

    row.set_activatable(false);

    if let Some(img) = ui::utils::file_icon(&shared::absolute(file), gtk::IconSize::Dnd) {
        row.add_prefix(&img);
    }

    row.set_title(file.to_str());

    let button = gtk::Button::new();
    button.add(&gtk::Image::from_icon_name(
        Some("edit-delete-symbolic"),
        gtk::IconSize::Button,
    ));
    button.add_css_class("image-button");
    row.add(&button);
    button.set_valign(gtk::Align::Center);

    button
}

// TODO: Function has too many lines
pub fn refresh() {
    let include_home = SETTINGS
        .get()
        .backups
        .get_active()
        .unwrap()
        .include
        .contains(&std::path::PathBuf::new());

    main_ui().include_home().set_sensitive(false);
    main_ui().include_home().set_active(include_home);
    main_ui().include_home().set_sensitive(true);

    if include_home {
        main_ui().include_home_row().remove_css_class("not-active");
    } else {
        main_ui().include_home_row().add_css_class("not-active");
    }

    let backup = SETTINGS.load().backups.get_active().unwrap().clone();

    // backup target ui
    let repo_ui = main_ui().target_listbox();

    if let Ok(icon) = gio::Icon::new_for_string(&utils::repo_icon(&backup.repo)) {
        main_ui()
            .detail_repo_icon()
            .set_from_gicon(&icon, gtk::IconSize::Dnd);
    }

    match &backup.repo {
        shared::BackupRepo::Local {
            ref device,
            ref label,
            ..
        } => {
            main_ui()
                .detail_repo_row()
                .set_title(label.as_ref().map(String::as_str));
            main_ui().detail_repo_row().set_subtitle(Some(
                device
                    .as_ref()
                    .map(String::as_str)
                    .unwrap_or(&backup.repo.to_string()),
            ));
        }
        repo @ shared::BackupRepo::Remote { .. } => {
            main_ui()
                .detail_repo_row()
                .set_title(Some(&gettext("Remote location")));
            main_ui()
                .detail_repo_row()
                .set_subtitle(Some(&repo.to_string()));
        }
    }

    repo_ui.show_all();

    // include list
    ui::utils::clear(&main_ui().include());
    // TODO: Warn if there a no includes, disable backup button
    for file in backup.include.iter() {
        if *file == std::path::PathBuf::new() {
            continue;
        }

        let button = add_list_row(&main_ui().include(), file);

        let path = file.clone();
        button.connect_clicked(move |_| {
            let delete =
                ui::utils::dialog_yes_no(gettext("You no longer want to backup this directory?"));

            if delete {
                SETTINGS.update(|settings| {
                    settings
                        .backups
                        .get_active_mut()
                        .unwrap()
                        .include
                        .remove(&path);
                });
                super::write_config();
                refresh();
            }
        });
    }
    main_ui().include().show_all();

    // exclude list
    ui::utils::clear(&main_ui().backup_exclude());
    for shared::Pattern::PathPrefix(file) in backup.exclude.iter() {
        let button = add_list_row(&main_ui().backup_exclude(), file);
        let path = file.clone();
        button.connect_clicked(move |_| {
            let path = path.clone();
            SETTINGS.update(move |settings| {
                settings
                    .backups
                    .get_active_mut()
                    .unwrap()
                    .exclude
                    .remove(&shared::Pattern::PathPrefix(path.clone()));
            });
            super::write_config();
            refresh();
        });
    }
    main_ui().backup_exclude().show_all();
    if backup.exclude.is_empty() {
        main_ui()
            .detail_exclude_stack()
            .set_visible_child(&main_ui().detail_exclude_placeholder());
    } else {
        main_ui()
            .detail_exclude_stack()
            .set_visible_child(&main_ui().backup_exclude());
    }

    refresh_statusx();
}

fn on_transition(stack: &gtk::Stack) {
    if (!stack.get_transition_running())
        && stack.get_visible_child() != Some(main_ui().page_detail().upcast::<gtk::Widget>())
    {
        // scroll back to top
        for scrollable in &[main_ui().page_backup(), main_ui().page_archives()] {
            scrollable
                .get_vadjustment()
                .unwrap()
                .set_value(scrollable.get_vadjustment().unwrap().get_lower());
        }
    }
}

/// Returns a relative path for sub directories of home
fn rel_path(path: &std::path::Path) -> std::path::PathBuf {
    if let Ok(rel_path) = path.strip_prefix(shared::get_home_dir()) {
        rel_path.to_path_buf()
    } else {
        path.to_path_buf()
    }
}

fn add_include() {
    if let Some(path) = ui::utils::folder_chooser_dialog(&gettext("Include directory in backup")) {
        SETTINGS.update(|settings| {
            settings
                .backups
                .get_active_mut()
                .unwrap()
                .include
                .insert(rel_path(&path));
        });
        super::write_config();
        refresh();
    }
}

fn add_exclude() {
    if let Some(path) = ui::utils::folder_chooser_dialog(&gettext("Exclude directory from backup"))
    {
        SETTINGS.update(|settings| {
            settings
                .backups
                .get_active_mut()
                .unwrap()
                .exclude
                .insert(shared::Pattern::PathPrefix(rel_path(&path)));
        });
        super::write_config();
        refresh();
    }
}

pub fn refresh_statusx() {
    main_ui().backup_run().set_sensitive(true);
    if let Some(communication) = BACKUP_COMMUNICATION.load().get_active() {
        main_ui().backup_run().set_sensitive(false);
        refresh_status(communication);
    } else if let Some(backup) = SETTINGS.load().backups.get_active() {
        refresh_offline(&backup.last_run);
    }
}

pub fn refresh_offline(run_info_opt: &Option<shared::RunInfo>) {
    let stack = main_ui().stack();
    main_ui().stop_backup_create().hide();

    if let Some(ref run_info) = run_info_opt {
        match &run_info.result {
            Ok(stats) => {
                main_ui().status_icon().set_visible_child_name("success");
                set_status(&gettext("Last backup successful"));

                // TODO: Translate durations
                set_status_detail(&format!(
                    "About {}",
                    (run_info.end - Local::now()).humanize()
                ));

                stack.set_visible_child_name("archive");
                main_ui().archive_progress().set_fraction(1.0);
                main_ui()
                    .original_size()
                    .set_text(&ui::utils::hsize(stats.archive.stats.original_size));
                main_ui()
                    .deduplicated_size()
                    .set_text(&ui::utils::hsize(stats.archive.stats.deduplicated_size));
            }
            Err(err) => {
                main_ui().status_icon().set_visible_child_name("error");
                set_status(&gettext("Last backup failed"));
                // TODO: Translate durations
                set_status_detail(&format!(
                    "About {}",
                    (run_info.end - Local::now()).humanize()
                ));
                stack.set_visible_child_name("error");
                main_ui().error_message().set_text(err);
            }
        }
    } else {
        main_ui().status_icon().set_visible_child_name("unknown");
        set_status(&gettext("Backup never ran"));
        set_status_detail(&gettext("Start by creating your first backup"));
    }
}

pub fn refresh_status(communication: &borg::Communication) -> Continue {
    main_ui().stop_backup_create().show();

    let stack = main_ui().stack();
    unset_status_detail();

    let status = communication.status.get();

    if let Some(ref last_message) = status.last_message {
        match *last_message {
            Progress::Archive {
                original_size,
                deduplicated_size,
                ref path,
                ..
            } => {
                stack.set_visible_child_name("archive");

                if let Some(total) = status.estimated_size {
                    let fraction = original_size as f64 / total as f64;
                    main_ui().archive_progress().show();
                    main_ui().archive_progress().set_fraction(fraction);
                    set_status_detail(&gettextf(
                        // xgettext:no-c-format
                        "{} % finished",
                        &[&format!("{:.1}", fraction * 100.0)],
                    ))
                } else {
                    main_ui().archive_progress().hide();
                }

                main_ui()
                    .original_size()
                    .set_text(&ui::utils::hsize(original_size));
                main_ui()
                    .deduplicated_size()
                    .set_text(&ui::utils::hsize(deduplicated_size));
                main_ui().current_path().set_text(path);
                main_ui().current_path().set_tooltip_text(Some(path));
            }
            Progress::Message {
                message: Some(ref message),
                ref msgid,
                ..
            } => {
                stack.set_visible_child_name("message");
                main_ui().message().set_text(message);
                if msgid.as_ref().map(|x| x.starts_with("cache.")) == Some(true) {
                    set_status_detail(&gettext("Updating repository information"));
                } else {
                    set_status(message);
                }
            }
            Progress::Percent {
                current: Some(current),
                total: Some(total),
                message: Some(ref message),
                ..
            } => {
                stack.set_visible_child_name("percent");
                let fraction = current as f64 / total as f64;
                main_ui().progress().set_fraction(fraction);
                main_ui().percent_message().set_text(message);
                set_status_detail(&gettextf(
                    // xgettext:no-c-format
                    "{} % prepared",
                    &[&format!("{:.1}", fraction * 100.0)],
                ))
            }
            // TODO: cover progress message?
            _ => {}
        }
    }

    main_ui().status_icon().set_visible_child_name("running");

    match status.run {
        Run::Init => {
            set_status(&gettext("Preparing backup"));
        }
        Run::SizeEstimation => {
            set_status(&gettext("Estimating backup size"));
        }
        Run::Running => {
            set_status(&gettext("Backup running"));
        }
        Run::Reconnecting => {
            set_status(&gettext("Reconnecting"));
            set_status_detail(&gettextf(
                "Connection lost, reconnecting in {}",
                &[&crate::BORG_DELAY_RECONNECT.humanize()],
            ));
        }
        Run::Stopping => {
            set_status(&gettext("Stopping backup"));
        }
    };

    Continue(true)
}

fn set_status(text: &str) {
    main_ui().detail_status_row().set_title(Some(text));
}

fn set_status_detail(text: &str) {
    main_ui().detail_status_row().set_subtitle(Some(text));
}

fn unset_status_detail() {
    main_ui().detail_status_row().set_subtitle(None);
}
