use gio::prelude::*;
use gtk::prelude::*;
use libhandy::prelude::*;

use crate::borg;
use crate::shared;
use crate::ui;
use crate::ui::backup_status;
use crate::ui::globals::*;
use crate::ui::prelude::*;

pub fn init() {
    main_ui().backup_run().connect_clicked(|_| on_backup_run());

    main_ui()
        .detail_status_row()
        .add_prefix(&main_ui().status_graphic());

    // Backup details
    main_ui().detail_status_row().set_activatable(true);
    main_ui()
        .detail_status_row()
        .connect_activated(|_| main_ui().detail_running_backup_info().show_all());

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
        .main_stack()
        .connect_property_visible_child_notify(on_stack_changed);
    main_ui()
        .detail_stack()
        .connect_property_visible_child_notify(on_stack_changed);

    main_ui()
        .include_home()
        .connect_property_active_notify(|switch| {
            if switch.get_sensitive() {
                let change: bool = if switch.get_active() {
                    true
                } else {
                    ui::utils::confirmation_dialog(
                &gettextf(
                    "No longer include “{}” in backups?",
                    &[&gettext("Home")],
                ),
                &gettext(
                    "All files contained in this folder will no longer be part of future backups.",
                ),
                &gettext("Cancel"),
                &gettext("Confirm"),
            )
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

    main_ui().status_spinner().connect_map(|s| s.start());
    main_ui().status_spinner().connect_unmap(|s| s.stop());

    glib::timeout_add_seconds_local(1, || {
        refresh_status();
        Continue(true)
    });
}

fn is_visible() -> bool {
    main_ui().detail_stack().get_visible_child()
        == Some(main_ui().page_backup().upcast::<gtk::Widget>())
        && main_ui().main_stack().get_visible_child()
            == Some(main_ui().page_detail().upcast::<gtk::Widget>())
}

fn on_stack_changed(_stack: &gtk::Stack) {
    if is_visible() {
        refresh_status();
    }
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
    if !ui::utils::confirmation_dialog(
        &gettext("Abort running backup creation?"),
        &gettext("The backup will remain incomplete if aborted now."),
        &gettext("Continue"),
        &gettext("Abort"),
    ) {
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

        let unmount = ui::utils::confirmation_dialog(
            &gettext("Stop browsing files and start backup?"),
            &gettext("Browsing through archived files is not possible while running a backup."),
            &gettext("Keep Browsing"),
            &gettext("Start Backup"),
        );
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

    ui::dialog_device_missing::main(config.clone(), "", move || run_backup(config.clone()));
}

pub fn run_backup(config: shared::BackupConfig) {
    let communication: borg::Communication = Default::default();

    BACKUP_COMMUNICATION.update(|x| {
        x.insert(config.id.clone(), communication.clone());
    });
    refresh_status();

    ui::utils::Async::borg(
        "borg::create",
        borg::Borg::new(config.clone()),
        move |borg| borg.create(communication),
        move |result| {
            BACKUP_COMMUNICATION.update(|c| {
                c.remove(&config.id);
            });
            let user_aborted = matches!(result, Err(shared::BorgErr::UserAborted));
            // This is because the error cannot be cloned
            let result_string_err = result.map_err(|err| format!("{}", err));
            let run_info = Some(shared::RunInfo::new(result_string_err.clone()));

            SETTINGS.update(|settings| {
                settings.backups.get_mut(&config.id).unwrap().last_run = run_info.clone()
            });
            refresh_status();

            ui::write_config();

            if !user_aborted {
                if let Err(err) = result_string_err {
                    ui::utils::show_error(gettext("Creating a backup failed."), err);
                } else {
                    ui::page_archives::refresh_archives_cache(config.clone());
                }
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

    if let Ok(icon) = gio::Icon::new_for_string(&backup.repo.icon()) {
        main_ui()
            .detail_repo_icon()
            .set_from_gicon(&icon, gtk::IconSize::Dnd);
    }

    match &backup.repo {
        shared::BackupRepo::Local { ref mount_name, .. } => {
            main_ui()
                .detail_repo_row()
                .set_title(mount_name.as_ref().map(String::as_str));
        }
        shared::BackupRepo::Remote { .. } => {
            main_ui()
                .detail_repo_row()
                .set_title(Some(&gettext("Remote location")));
        }
    }

    main_ui()
        .detail_repo_row()
        .set_subtitle(Some(&backup.repo.get_subtitle()));

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
            let delete = ui::utils::confirmation_dialog(
                &gettextf(
                    "No longer include “{}” in backups?",
                    &[&path.to_string_lossy()],
                ),
                &gettext(
                    "All files contained in this folder will no longer be part of future backups.",
                ),
                &gettext("Cancel"),
                &gettext("Confirm"),
            );

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
}

fn on_transition(stack: &gtk::Stack) {
    if !stack.get_transition_running() && !is_visible() {
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
    if let Some(path) =
        ui::utils::folder_chooser_dialog_path(&gettext("Include directory in backups"))
    {
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
    if let Some(path) =
        ui::utils::folder_chooser_dialog_path(&gettext("Exclude directory from backup"))
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

fn refresh_status() {
    if is_visible() {
        if let Some(id) = ACTIVE_BACKUP_ID.load().as_ref().as_ref() {
            refresh_status_display(&backup_status::Display::new_from_id(id));
        }
    }
}

fn refresh_status_display(status: &ui::backup_status::Display) {
    main_ui().detail_status_row().set_title(Some(&status.title));
    main_ui()
        .detail_status_row()
        .set_subtitle(status.subtitle.as_deref());

    let running = match &status.graphic {
        ui::backup_status::Graphic::ErrorIcon(icon) | ui::backup_status::Graphic::Icon(icon) => {
            main_ui()
                .status_graphic()
                .set_visible_child(&main_ui().status_icon());
            main_ui()
                .status_icon()
                .set_from_icon_name(Some(icon), gtk::IconSize::Dnd);

            false
        }
        ui::backup_status::Graphic::Spinner => {
            main_ui()
                .status_graphic()
                .set_visible_child(&main_ui().status_spinner());

            true
        }
    };

    if matches!(status.graphic, ui::backup_status::Graphic::ErrorIcon(_)) {
        main_ui().status_icon().add_css_class("error");
        main_ui().detail_hint_icon().show();
    } else {
        main_ui().status_icon().remove_css_class("error");
        main_ui().detail_hint_icon().hide();
    }

    main_ui().stop_backup_create().set_visible(running);
    main_ui().backup_run().set_sensitive(!running);
}
