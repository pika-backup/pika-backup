use chrono::prelude::*;
use gio::prelude::*;
use gtk::prelude::*;

use crate::borg;
use crate::borg::Run;
use crate::shared;
use crate::shared::*;
use crate::ui;
use crate::ui::globals::*;
use crate::ui::prelude::*;
use crate::ui::utils::{self, WidgetEnh};

pub fn init() {
    main_ui().backup_run().connect_clicked(|_| on_backup_run());

    main_ui()
        .target_listbox()
        .connect_row_activated(|_, _| ui::storage::show());

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

    main_ui().include().connect_row_activated(|_, row| {
        if row.get_widget_name() == "-add" {
            add_include();
        }
    });

    main_ui().backup_exclude().connect_row_activated(|_, row| {
        if row.get_widget_name() == "-add" {
            add_exclude();
        }
    });

    main_ui().delete_backup_conf().connect_clicked(|_| {
        let delete = ui::utils::dialog_yes_no(gettext(
            "Are you sure you want to delete this backup configuration?",
        ));

        if delete {
            ui::utils::dialog_catch_err(
                ui::utils::secret_service_delete_passwords(
                    SETTINGS.get().backups.get_active().unwrap(),
                ),
                "Failed to remove potentially remaining passwords from key storage.",
            );
            {
                SETTINGS.update(|settings| {
                    settings.backups.remove(&ACTIVE_BACKUP_ID.get().unwrap());
                });
            }

            super::write_config();
            ui::overview::refresh();
            main_ui().main_stack().set_visible_child_name("main");
        }
    });

    main_ui()
        .stop_backup_create()
        .connect_clicked(|_| stop_backup_create());

    glib::timeout_add_local(500, || {
        refresh_statusx();
        Continue(true)
    });
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

    ui::device_missing::main(config.clone(), move || run_backup(config.clone()));
}

pub fn run_backup(backup: shared::BackupConfig) {
    let backup_id = backup.id.clone();

    let communication: borg::Communication = Default::default();

    BACKUP_COMMUNICATION.update(|x| {
        x.insert(backup_id.clone(), communication.clone());
    });
    refresh_status(&communication);

    ui::utils::Async::borg(
        "borg::create",
        borg::Borg::new(backup),
        move |borg| borg.create(communication),
        move |result| {
            BACKUP_COMMUNICATION.update(|c| {
                c.remove(&backup_id);
            });
            let user_aborted = matches!(result, Err(shared::BorgErr::UserAborted));
            let result_string_err = result.map_err(|err| format!("{}", err));
            let run_info = Some(shared::RunInfo::new(result_string_err.clone()));
            refresh_offline(&run_info);
            SETTINGS.update(|settings| {
                settings.backups.get_mut(&backup_id).unwrap().last_run = run_info.clone()
            });
            ui::write_config();

            if !user_aborted {
                ui::utils::dialog_catch_errb(&result_string_err, gettext("Backup failed"));
            }
        },
    );
}

pub fn add_list_row(list: &gtk::ListBox, file: &std::path::Path, position: i32) -> gtk::Button {
    let (row, horizontal_box) = ui::utils::add_list_box_row(list, None, position);

    row.set_activatable(false);

    if let Some(img) = ui::utils::file_icon(&shared::absolute(file), gtk::IconSize::Dialog) {
        horizontal_box.add(&img);
    }

    let label = gtk::Label::new(file.to_str());
    label.set_line_wrap(true);
    label.set_line_wrap_mode(pango::WrapMode::WordChar);
    label.set_xalign(0.0);
    horizontal_box.add(&label);

    let button = gtk::Button::new();
    button.add(&gtk::Image::from_icon_name(
        Some("window-close-symbolic"),
        gtk::IconSize::Button,
    ));
    button.add_css_class("circular");
    button.set_valign(gtk::Align::Center);
    horizontal_box.pack_end(&button, false, false, 0);

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
        main_ui().home_icon().remove_css_class("not-active");
    } else {
        main_ui().home_icon().add_css_class("not-active");
    }

    let backup = SETTINGS.load().backups.get_active().unwrap().clone();

    // backup target ui
    let repo_ui = main_ui().target_listbox();
    ui::utils::clear(&repo_ui);
    let (_, horizontal_box) = ui::utils::add_list_box_row(&repo_ui, None, 1);

    if let Ok(icon) = gio::Icon::new_for_string(&utils::repo_icon(&backup.repo)) {
        let img = gtk::Image::from_gicon(&icon, gtk::IconSize::Dialog);
        horizontal_box.add(&img);
    }

    match &backup.repo {
        shared::BackupRepo::Local {
            ref device,
            ref label,
            ..
        } => {
            let (vertical_box, _, _) = ui::utils::list_vertical_box(
                label.as_ref().map(String::as_str),
                Some(
                    device
                        .as_ref()
                        .map(String::as_str)
                        .unwrap_or(&backup.repo.to_string()),
                ),
            );
            horizontal_box.add(&vertical_box);
        }
        repo @ shared::BackupRepo::Remote { .. } => {
            let (vertical_box, _, _) = ui::utils::list_vertical_box(Some(&repo.to_string()), None);
            horizontal_box.add(&vertical_box);
        }
    }

    repo_ui.show_all();

    // include list
    ui::utils::clear(&main_ui().include());
    // TODO: Warn if there a no includes, disable backup button
    for file in backup.include.iter().rev() {
        if *file == std::path::PathBuf::new() {
            continue;
        }

        let button = add_list_row(&main_ui().include(), file, 1);

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
    for shared::Pattern::PathPrefix(file) in backup.exclude.iter().rev() {
        let button = add_list_row(&main_ui().backup_exclude(), file, 0);
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

    refresh_statusx();
}

pub fn view_backup_conf(id: &str) {
    ACTIVE_BACKUP_ID.update(|active_id| *active_id = Some(id.to_string()));
    refresh();
    // scroll back to top
    if let Some(adjustment) = main_ui().detail_scrolled().get_vadjustment() {
        adjustment.set_value(adjustment.get_lower());
    }
    main_ui().main_stack().set_visible_child_name("backup_conf");
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
        main_ui().status_button().show();
        match &run_info.result {
            Ok(stats) => {
                main_ui().status_icon().set_visible_child_name("success");
                main_ui()
                    .status_text()
                    .set_text(&gettext("Last backup successful"));

                main_ui().status_subtext().set_text(&gettext!(
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
                main_ui()
                    .status_text()
                    .set_text(&gettext("Last backup failed"));
                main_ui().status_subtext().set_text(&gettext!(
                    "About {}",
                    (run_info.end - Local::now()).humanize()
                ));
                stack.set_visible_child_name("error");
                main_ui().error_message().set_text(err);
            }
        }
    } else {
        main_ui().status_button().hide();
        main_ui().status_icon().set_visible_child_name("unknown");
        main_ui()
            .status_text()
            .set_text(&gettext("Backup never ran"));
        main_ui().status_subtext().set_text("");
    }
}

pub fn refresh_status(communication: &borg::Communication) -> Continue {
    main_ui().stop_backup_create().show();

    let stack = main_ui().stack();
    main_ui().status_subtext().set_text("");

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
                    main_ui()
                        .status_subtext()
                        .set_text(&gettext!("{:.1} % finished", fraction * 100.0))
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
                    main_ui()
                        .status_subtext()
                        .set_text(&gettext("Updating repository information"));
                } else {
                    main_ui().status_subtext().set_text(message);
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
                main_ui()
                    .status_subtext()
                    .set_text(&gettext!("{:.1} % prepared", fraction * 100.0))
            }
            // TODO: cover progress message?
            _ => {}
        }
    }

    main_ui().status_icon().set_visible_child_name("running");

    match status.run {
        Run::Init => {
            main_ui().status_button().hide();
            main_ui()
                .status_text()
                .set_text(&gettext("Preparing backup"));
        }
        Run::SizeEstimation => {
            main_ui().status_button().hide();
            main_ui()
                .status_text()
                .set_text(&gettext("Estimating backup size"));
        }
        Run::Running => {
            main_ui().status_button().show();
            main_ui().status_text().set_text(&gettext("Backup running"));
        }
        Run::Reconnecting => {
            main_ui().status_button().show();
            main_ui().status_text().set_text(&gettext("Reconnecting"));
            main_ui().status_subtext().set_text(&gettext!(
                "Connection lost, reconnecting {}",
                &crate::BORG_DELAY_RECONNECT.humanize()
            ));
        }
        Run::Stopping => {
            main_ui().status_button().show();
            main_ui()
                .status_text()
                .set_text(&gettext("Stopping backup"));
        }
    };

    Continue(true)
}
