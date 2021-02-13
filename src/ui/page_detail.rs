use gio::prelude::*;
use gtk::prelude::*;
use libhandy::prelude::*;

use crate::borg;
use crate::config;
use crate::ui;
use crate::ui::backup_status;
use crate::ui::globals::*;
use crate::ui::prelude::*;

pub fn init() {
    main_ui()
        .backup_run()
        .connect_clicked(|_| Handler::run(on_backup_run()));

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
        .connect_activated(|_| spawn_local(ui::dialog_storage::show()));

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
        .connect_property_active_notify(|_| {
            Handler::run(async move {
                if main_ui().include_home().get_sensitive() {
                    let change: bool = if main_ui().include_home().get_active() {
                        true
                    } else {
                        confirm_remove_include(std::path::Path::new("Home")).await
                    };

                    SETTINGS.update(|settings| {
                        if !change {
                            main_ui()
                                .include_home()
                                .set_active(!main_ui().include_home().get_active());
                        } else if main_ui().include_home().get_active() {
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
                        super::write_config()?;
                        refresh()?;
                    }
                } else {
                    main_ui().include_home().set_sensitive(true);
                }

                Ok(())
            })
        });

    main_ui()
        .add_include()
        .connect_clicked(|_| Handler::run(add_include()));
    main_ui()
        .add_exclude()
        .connect_clicked(|_| Handler::run(add_exclude()));

    main_ui()
        .stop_backup_create()
        .connect_clicked(|_| Handler::run(on_stop_backup_create()));

    main_ui().status_spinner().connect_map(|s| s.start());
    main_ui().status_spinner().connect_unmap(|s| s.stop());

    glib::timeout_add_seconds_local(1, || {
        refresh_status();
        Continue(true)
    });
}

pub fn activate_action_backup(id: ConfigId) {
    Handler::run(
        async move { start_backup(SETTINGS.load().backups.get_result(&id)?.clone()).await },
    );
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

pub fn view_backup_conf(id: &ConfigId) {
    ACTIVE_BACKUP_ID.update(|active_id| *active_id = Some(id.clone()));
    Handler::run(async { refresh() });

    main_ui()
        .detail_stack()
        .set_visible_child(&main_ui().page_backup());
    main_ui()
        .main_stack()
        .set_visible_child(&main_ui().page_detail());
}

async fn on_stop_backup_create() -> Result<()> {
    ui::utils::confirmation_dialog(
        &gettext("Abort running backup creation?"),
        &gettext("The backup will remain incomplete if aborted now."),
        &gettext("Continue"),
        &gettext("Abort"),
    )
    .await?;

    if let Some(communication) = BACKUP_COMMUNICATION.load().get_active() {
        communication.instruction.update(|inst| {
            *inst = borg::Instruction::Abort;
        });
    }

    Ok(())
}

async fn on_backup_run() -> Result<()> {
    start_backup(SETTINGS.load().backups.get_active_result()?.clone()).await
}

async fn start_backup(config: config::BackupConfig) -> Result<()> {
    gtk_app().hold();
    let result = startup_backup(config).await;
    gtk_app().release();
    result
}

async fn startup_backup(config: config::BackupConfig) -> Result<()> {
    let is_running_on_repo = BACKUP_COMMUNICATION.get().keys().any(|id| {
        SETTINGS.get().backups.get(id).map(|config| &config.repo_id) == Some(&config.repo_id)
    });
    if is_running_on_repo {
        return Err(Message::short(gettext("Backup on repository already running.")).into());
    }

    if ACTIVE_MOUNTS.load().contains(&config.repo_id) {
        debug!("Trying to run borg::create on a backup that is currently mounted.");

        ui::utils::confirmation_dialog(
            &gettext("Stop browsing files and start backup?"),
            &gettext("Browsing through archived files is not possible while running a backup."),
            &gettext("Keep Browsing"),
            &gettext("Start Backup"),
        )
        .await?;

        trace!("User decided to unmount repo.");
        borg::Borg::umount(&config.repo_id).err_to_msg(gettext("Failed to unmount repository."))?;

        ACTIVE_MOUNTS.update(|mounts| {
            mounts.remove(&config.repo_id);
        });
    }

    let config =
        ui::dialog_device_missing::updated_config(config, &gettext("Creating a new backup."))
            .await?;

    run_backup(config).await
}

pub async fn run_backup(config: config::BackupConfig) -> Result<()> {
    let communication: borg::Communication = Default::default();

    // skip size estimate if running in background
    if !crate::ui::app_window::is_displayed() {
        communication.instruction.update(|inst| {
            *inst = borg::Instruction::AbortSizeEstimation;
        });
    }

    BACKUP_COMMUNICATION.update(|x| {
        x.insert(config.id.clone(), communication.clone());
    });
    refresh_status();

    let result = ui::utils::borg::spawn(
        "borg::create",
        borg::Borg::new(config.clone()),
        move |borg| borg.create(communication),
    )
    .await;

    BACKUP_COMMUNICATION.update(|c| {
        c.remove(&config.id);
    });
    let user_aborted = matches!(result, Err(borg::Error::UserAborted));
    // This is because the error cannot be cloned
    let result_config = match result {
        Err(borg::Error::BorgCreate(err)) => Err(config::RunError::WithLevel {
            message: format!("{}", err),
            level: err.level,
            stats: err.stats,
        }),
        Err(err) => Err(config::RunError::Simple(format!("{}", err))),
        Ok(stats) => Ok(stats),
    };
    let run_info = Some(config::RunInfo::new(result_config.clone()));

    SETTINGS.update(|settings| {
        settings.backups.get_mut(&config.id).unwrap().last_run = run_info.clone()
    });
    refresh_status();

    ui::write_config()?;

    if !user_aborted {
        if let Err(err) = result_config {
            if err.level() >= borg::msg::LogLevel::ERROR {
                return Err(Message::new(gettext("Creating a backup failed."), err).into());
            } else {
                return Err(Message::new(gettext("Backup completed with warnings."), err).into());
            }
        } else {
            ui::page_archives::refresh_archives_cache(config.clone()).await?;
        }
    }

    Ok(())
}

pub fn add_list_row(list: &gtk::ListBox, file: &std::path::Path) -> gtk::Button {
    let row = libhandy::ActionRowBuilder::new()
        .title(&file.to_string_lossy())
        .activatable(false)
        .build();
    list.add(&row);

    if let Some(img) = ui::utils::file_icon(&config::absolute(file), gtk::IconSize::Dnd) {
        row.add_prefix(&img);
    }

    let button = gtk::ButtonBuilder::new()
        .child(&gtk::Image::from_icon_name(
            Some("edit-delete-symbolic"),
            gtk::IconSize::Button,
        ))
        .build();
    button.add_css_class("image-button");
    row.add(&button);
    button.set_valign(gtk::Align::Center);

    button
}

// TODO: Function has too many lines
pub fn refresh() -> Result<()> {
    let backup = SETTINGS.load().backups.get_active_result()?.clone();

    let include_home = backup.include.contains(&std::path::PathBuf::new());

    if include_home != main_ui().include_home().get_active() {
        main_ui().include_home().set_sensitive(false);
        main_ui().include_home().set_active(include_home);
    }

    if include_home {
        main_ui().include_home_row().remove_css_class("not-active");
    } else {
        main_ui().include_home_row().add_css_class("not-active");
    }

    // backup target ui
    let repo_ui = main_ui().target_listbox();

    if let Ok(icon) = gio::Icon::new_for_string(&backup.repo.icon()) {
        main_ui()
            .detail_repo_icon()
            .set_from_gicon(&icon, gtk::IconSize::Dnd);
    }

    match &backup.repo {
        config::BackupRepo::Local(local) => {
            main_ui()
                .detail_repo_row()
                .set_title(local.mount_name.as_deref());
        }
        config::BackupRepo::Remote(_) => {
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
            let path = path.clone();
            Handler::run(async move {
                if confirm_remove_include(&path).await {
                    SETTINGS.update(|settings| {
                        settings
                            .backups
                            .get_active_mut()
                            .unwrap()
                            .include
                            .remove(&path);
                    });
                    super::write_config()?;
                    refresh()?;
                }

                Ok(())
            })
        });
    }
    main_ui().include().show_all();

    // exclude list
    ui::utils::clear(&main_ui().backup_exclude());
    for config::Pattern::PathPrefix(file) in backup.exclude.clone().into_iter() {
        let button = add_list_row(&main_ui().backup_exclude(), &file);
        button.connect_clicked(move |_| {
            let path = file.clone();
            Handler::run(async move {
                let path = path.clone();
                SETTINGS.update(move |settings| {
                    settings
                        .backups
                        .get_active_mut()
                        .unwrap()
                        .exclude
                        .remove(&config::Pattern::PathPrefix(path.clone()));
                });
                super::write_config()?;
                refresh()?;
                Ok(())
            });
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

    Ok(())
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

async fn confirm_remove_include(path: &std::path::Path) -> bool {
    ui::utils::confirmation_dialog(
        &gettextf(
            "No longer include “{}” in backups?",
            &[&path.to_string_lossy()],
        ),
        &gettext("All files contained in this folder will no longer be part of future backups."),
        &gettext("Cancel"),
        &gettext("Confirm"),
    )
    .await
    .is_ok()
}

/// Returns a relative path for sub directories of home
fn rel_path(path: &std::path::Path) -> std::path::PathBuf {
    if let Ok(rel_path) = path.strip_prefix(HOME_DIR.as_path()) {
        rel_path.to_path_buf()
    } else {
        path.to_path_buf()
    }
}

async fn add_include() -> Result<()> {
    if let Some(path) =
        ui::utils::folder_chooser_dialog_path(&gettext("Include directory in backups")).await
    {
        SETTINGS.update(|settings| {
            settings
                .backups
                .get_active_mut()
                .unwrap()
                .include
                .insert(rel_path(&path));
        });
        super::write_config()?;
        refresh()?;
    }

    Ok(())
}

async fn add_exclude() -> Result<()> {
    if let Some(path) =
        ui::utils::folder_chooser_dialog_path(&gettext("Exclude directory from backup")).await
    {
        SETTINGS.update(|settings| {
            settings
                .backups
                .get_active_mut()
                .unwrap()
                .exclude
                .insert(config::Pattern::PathPrefix(rel_path(&path)));
        });
        super::write_config()?;
        refresh()?;
    }

    Ok(())
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
        ui::backup_status::Graphic::ErrorIcon(icon)
        | ui::backup_status::Graphic::WarningIcon(icon)
        | ui::backup_status::Graphic::Icon(icon) => {
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
        main_ui().status_icon().remove_css_class("warning");
        main_ui().detail_hint_icon().show();
    } else if matches!(status.graphic, ui::backup_status::Graphic::WarningIcon(_)) {
        main_ui().status_icon().add_css_class("warning");
        main_ui().status_icon().remove_css_class("error");
    } else {
        main_ui().status_icon().remove_css_class("error");
        main_ui().status_icon().remove_css_class("warning");
        main_ui().detail_hint_icon().hide();
    }

    main_ui().stop_backup_create().set_visible(running);
    main_ui().backup_run().set_sensitive(!running);
}
