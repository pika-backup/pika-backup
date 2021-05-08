use gio::prelude::*;
use gtk::prelude::*;
use libhandy::prelude::*;

use crate::borg;
use crate::config;
use crate::config::history;
use crate::ui;
use crate::ui::backup_status;
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
        .connect_activated(|_| Handler::run(ui::dialog_storage::show()));

    main_ui()
        .main_stack()
        .connect_transition_running_notify(on_transition);
    main_ui()
        .main_stack()
        .connect_visible_child_notify(on_stack_changed);
    main_ui()
        .detail_stack()
        .connect_visible_child_notify(on_stack_changed);

    main_ui().include_home().connect_active_notify(|_| {
        Handler::run(async move {
            if main_ui().include_home().is_sensitive() {
                let change: bool = if main_ui().include_home().is_active() {
                    true
                } else {
                    confirm_remove_include(std::path::Path::new("Home")).await
                };

                BACKUP_CONFIG.update_result(|settings| {
                    if !change {
                        main_ui()
                            .include_home()
                            .set_active(!main_ui().include_home().is_active());
                    } else if main_ui().include_home().is_active() {
                        settings
                            .active_mut()?
                            .include
                            .insert(std::path::PathBuf::new());
                    } else {
                        settings
                            .active_mut()?
                            .include
                            .remove(&std::path::PathBuf::new());
                    }
                    Ok(())
                })?;

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
    Handler::run(async move { start_backup(BACKUP_CONFIG.load().get_result(&id)?.clone()).await });
}

fn is_visible() -> bool {
    main_ui().detail_stack().visible_child()
        == Some(main_ui().page_backup().upcast::<gtk::Widget>())
        && main_ui().main_stack().visible_child()
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

    BACKUP_COMMUNICATION
        .load()
        .active()?
        .instruction
        .update(|inst| {
            *inst = borg::Instruction::Abort;
        });

    Ok(())
}

async fn on_backup_run() -> Result<()> {
    start_backup(BACKUP_CONFIG.load().active()?.clone()).await
}

async fn start_backup(config: config::Backup) -> Result<()> {
    gtk_app().hold();
    let result = startup_backup(config).await;
    gtk_app().release();
    result
}

async fn startup_backup(config: config::Backup) -> Result<()> {
    let is_running_on_repo = BACKUP_COMMUNICATION.get().keys().any(|id| {
        BACKUP_CONFIG
            .get()
            .get_result(id)
            .map(|config| &config.repo_id)
            == Ok(&config.repo_id)
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

    run_backup(config).await
}

pub async fn run_backup(config: config::Backup) -> Result<()> {
    let communication: borg::Communication = Default::default();

    BACKUP_COMMUNICATION.update(|x| {
        x.insert(config.id.clone(), communication.clone());
    });
    refresh_status();

    // estimate backup size if not running in background
    if crate::ui::app_window::is_displayed() {
        communication
            .status
            .update(|status| status.run = borg::Run::SizeEstimation);

        let estimated_size = ui::utils::spawn_thread(
            "estimate_backup_size",
            enclose!((config, communication) move ||
                borg::size_estimate::calculate(&config, &communication)
            ),
        )
        .await
        .ok()
        .flatten();

        if estimated_size.is_some() {
            communication.status.update(move |status| {
                status.estimated_size = estimated_size.clone();
            });
        }
    }

    let estimated_changed = communication
        .status
        .load()
        .estimated_size
        .as_ref()
        .map(|x| x.changed);
    let space_avail = ui::utils::df::cached_or_lookup(&config)
        .await
        .map(|x| x.avail);

    if let (Some(estimated_changed), Some(space_avail)) = (estimated_changed, space_avail) {
        if estimated_changed > space_avail {
            ui::utils::show_notice(gettextf(
                "Backup location “{}” might be filling up. Estimated space missing to store all data: {}.",
                &[
                    &config.repo.location(),
                    &glib::format_size(estimated_changed - space_avail),
                ],
            ));
        }
    }

    // execute backup
    let result = ui::utils::borg::exec(
        gettext("Creating new backup."),
        config.clone(),
        enclose!((communication) move |borg| borg.create(communication)),
    )
    .await;

    BACKUP_COMMUNICATION.update(|c| {
        c.remove(&config.id);
    });

    // Direct visual feedback for aborted backups
    if matches!(result, Err(ui::error::Combined::Ui(_))) {
        refresh_status();
    }

    let result = result.into_borg_error()?;

    // This is because the error cannot be cloned
    let outcome = match &result {
        Err(borg::Error::Aborted(err)) => borg::Outcome::Aborted(err.clone()),
        Err(borg::Error::Failed(err)) => borg::Outcome::Failed(err.clone()),
        Err(err) => borg::Outcome::Failed(borg::error::Failure::Other(err.to_string())),
        Ok(stats) => borg::Outcome::Completed {
            stats: stats.clone(),
        },
    };

    let message_history = communication.status.load().all_combined_message_history();

    let run_info = history::RunInfo::new(&config, outcome, message_history);

    BACKUP_HISTORY.update(|history| {
        history.insert(config.id.clone(), run_info.clone());
    });
    refresh_status();

    ui::write_config()?;

    match result {
        Err(borg::Error::Aborted(_)) => Ok(()),
        Err(err) => Err(Message::new(gettext("Creating a backup failed."), err).into()),
        Ok(_)
            if run_info.messages.clone().filter_handled().max_log_level()
                >= Some(borg::msg::LogLevel::Warning) =>
        {
            Err(Message::new(
                gettext("Backup completed with warnings."),
                run_info.messages.filter_hidden().to_string(),
            )
            .into())
        }
        Ok(_) => {
            let _ignore = ui::page_archives::refresh_archives_cache(config.clone()).await;
            let _ignore = ui::utils::df::lookup_and_cache(&config).await;
            Ok(())
        }
    }
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
    row.add(&button);
    button.set_valign(gtk::Align::Center);

    button
}

// TODO: Function has too many lines
pub fn refresh() -> Result<()> {
    let backup = BACKUP_CONFIG.load().active()?.clone();

    refresh_status();

    let include_home = backup.include.contains(&std::path::PathBuf::new());

    if include_home != main_ui().include_home().is_active() {
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

    if let Ok(icon) = gio::Icon::for_string(&backup.repo.icon()) {
        main_ui()
            .detail_repo_icon()
            .set_from_gicon(&icon, gtk::IconSize::Dnd);
    }

    match &backup.repo {
        config::Repository::Local(local) => {
            main_ui()
                .detail_repo_row()
                .set_title(local.mount_name.as_deref());
        }
        config::Repository::Remote(_) => {
            main_ui()
                .detail_repo_row()
                .set_title(Some(&gettext("Remote location")));
        }
    }

    main_ui()
        .detail_repo_row()
        .set_subtitle(Some(&backup.repo.subtitle()));

    repo_ui.show_all();

    // include list
    ui::utils::clear(&main_ui().include());
    // TODO: Warn if there a no includes, disable backup button
    for file in &backup.include {
        if *file == std::path::PathBuf::new() {
            continue;
        }

        let button = add_list_row(&main_ui().include(), file);

        let path = file.clone();
        button.connect_clicked(move |_| {
            let path = path.clone();
            Handler::run(async move {
                if confirm_remove_include(&path).await {
                    BACKUP_CONFIG.update_result(|settings| {
                        settings.active_mut()?.include.remove(&path);
                        Ok(())
                    })?;
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
    for pattern in backup.exclude.clone() {
        match pattern {
            config::Pattern::PathPrefix(file) => {
                let button = add_list_row(&main_ui().backup_exclude(), &file);
                button.connect_clicked(move |_| {
                    let path = file.clone();
                    Handler::run(async move {
                        BACKUP_CONFIG.update_result(move |settings| {
                            settings
                                .active_mut()?
                                .exclude
                                .remove(&config::Pattern::PathPrefix(path.clone()));
                            Ok(())
                        })?;
                        super::write_config()?;
                        refresh()?;
                        Ok(())
                    });
                });
            }
            config::Pattern::RegularExpression(regex) => {
                let row = libhandy::ActionRowBuilder::new()
                    .title(regex.as_str())
                    .subtitle(&gettext("Regular Expression"))
                    .activatable(false)
                    .icon_name("folder-saved-search")
                    .build();
                let button = gtk::ButtonBuilder::new()
                    .child(&gtk::Image::from_icon_name(
                        Some("edit-delete-symbolic"),
                        gtk::IconSize::Button,
                    ))
                    .build();
                button.set_valign(gtk::Align::Center);
                button.connect_clicked(move |_| {
                    let regex = regex.clone();
                    Handler::run(async move {
                        BACKUP_CONFIG.update_result(move |settings| {
                            settings
                                .active_mut()?
                                .exclude
                                .remove(&config::Pattern::RegularExpression(regex.clone()));
                            Ok(())
                        })?;
                        super::write_config()?;
                        refresh()?;
                        Ok(())
                    });
                });
                row.add(&button);
                main_ui().backup_exclude().add(&row);
            }
        }
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
    if !stack.is_transition_running() && !is_visible() {
        // scroll back to top
        for scrollable in &[main_ui().page_backup(), main_ui().page_archives()] {
            scrollable
                .vadjustment()
                .set_value(scrollable.vadjustment().lower());
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
    if let Ok(rel_path) = path.strip_prefix(glib::home_dir().as_path()) {
        rel_path.to_path_buf()
    } else {
        path.to_path_buf()
    }
}

async fn add_include() -> Result<()> {
    if let Some(path) =
        ui::utils::folder_chooser_dialog_path(&gettext("Include directory in backups")).await
    {
        BACKUP_CONFIG.update_result(|settings| {
            settings.active_mut()?.include.insert(rel_path(&path));
            Ok(())
        })?;
        super::write_config()?;
        refresh()?;
    }

    Ok(())
}

async fn add_exclude() -> Result<()> {
    if let Some(path) =
        ui::utils::folder_chooser_dialog_path(&gettext("Exclude directory from backup")).await
    {
        BACKUP_CONFIG.update_result(|settings| {
            settings
                .active_mut()?
                .exclude
                .insert(config::Pattern::PathPrefix(rel_path(&path)));
            Ok(())
        })?;
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
        main_ui().detail_hint_icon().show();
    } else {
        main_ui().status_icon().remove_css_class("error");
        main_ui().status_icon().remove_css_class("warning");
        main_ui().detail_hint_icon().hide();
    }

    main_ui().stop_backup_create().set_visible(running);
    main_ui().backup_run().set_sensitive(!running);
}
