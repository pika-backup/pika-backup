use std::{convert::Into, io::Read, rc::Rc};

use gio::prelude::*;
use gtk::prelude::*;
use zeroize::Zeroizing;

use crate::borg;
use crate::shared;
use crate::shared::*;
use crate::ui;
use crate::ui::builder;
use crate::ui::globals::*;
use crate::ui::prelude::*;
use ui::main_pending;

pub fn new_backup() {
    let ui_new = Rc::new(ui::builder::NewBackup::new());
    refresh(&ui_new);
    ui_new
        .password_quality()
        .add_offset_value(&gtk::LEVEL_BAR_OFFSET_LOW, 7.0);
    ui_new
        .password_quality()
        .add_offset_value(&gtk::LEVEL_BAR_OFFSET_HIGH, 5.0);
    ui_new
        .password_quality()
        .add_offset_value(&gtk::LEVEL_BAR_OFFSET_FULL, 3.0);

    ui_new
        .new_backup()
        .set_transient_for(Some(&main_ui().window()));

    let dialog = ui_new.new_backup();
    ui_new.cancel_button().connect_clicked(move |_| {
        dialog.close();
        dialog.hide();
    });

    let ui = ui_new.clone();
    ui_new
        .add_repo_list()
        .connect_row_activated(move |_, row| add_repo_list_activated(row, ui.clone()));

    let ui = ui_new.clone();
    ui_new
        .init_repo_list()
        .connect_row_activated(move |_, row| init_repo_list_activated(row, &ui));

    let ui = ui_new.clone();
    ui_new
        .password()
        .connect_changed(move |_| init_repo_password_changed(&ui));

    let ui = ui_new.clone();
    ui_new
        .add_button()
        .connect_clicked(move |_| add_button_clicked(ui.clone()));

    let ui = ui_new.clone();
    ui_new
        .init_button()
        .connect_clicked(move |_| init_button_clicked(ui.clone()));

    // refresh ui on mount events
    let monitor = gio::VolumeMonitor::get();

    let ui = ui_new.clone();
    monitor.connect_mount_added(move |_, _| refresh(&ui));
    let ui = ui_new.clone();
    monitor.connect_mount_changed(move |_, _| refresh(&ui));
    let ui = ui_new.clone();
    monitor.connect_mount_removed(move |_, _| refresh(&ui));

    ui_new.new_backup().show_all();
}

fn add_repo_list_activated(row: &gtk::ListBoxRow, ui: Rc<builder::NewBackup>) {
    if let Some(name) = row.get_widget_name() {
        if name == "-add-local" {
            add_local(ui);
        } else if name == "-add-remote" {
            ui.stack().set_visible_child(&ui.add_remote_page());
            ui.add_button().show();
            ui.add_button().grab_default();
        } else {
            add_repo_config_local(std::path::Path::new(&name), ui);
        }
    }
}

fn add_local(ui: Rc<builder::NewBackup>) {
    ui.new_backup().hide();

    if let Some(path) = ui::utils::folder_chooser_dialog(&gettext("Select existing repository")) {
        ui::main_pending::show(&gettext("Adding existing repository …"));
        if is_backup_repo(&path) {
            add_repo_config_local(&path, ui);
        } else {
            ui::utils::dialog_error(gettext(
                "The selected directory is not a valid backup repository.",
            ));
            ui::main_pending::back();
            ui.new_backup().show();
        }
    } else {
        ui.new_backup().show();
    }
}

fn add_button_clicked(ui: Rc<builder::NewBackup>) {
    main_pending::show(&gettext("Initializing new backup respository …"));
    ui.new_backup().hide();

    let uri = ui.add_remote_uri().get_text().unwrap();
    add_repo_config_remote(uri.to_string(), ui);
}

fn init_repo_list_activated(row: &gtk::ListBoxRow, ui: &builder::NewBackup) {
    if let Some(name) = row.get_widget_name() {
        ui.init_dir().set_text(&format!(
            "backup-{}-{}",
            glib::get_host_name()
                .map(|x| x.to_string())
                .unwrap_or_default(),
            glib::get_user_name()
                .and_then(|x| x.into_string().ok())
                .unwrap_or_default()
        ));
        if name == "-init-remote" {
            ui.init_location().set_visible_child(&ui.init_remote());
        } else {
            ui.init_location().set_visible_child(&ui.init_local());
            trace!("Setting {} as init_path", &name);
            ui.init_path()
                .set_current_folder(std::path::PathBuf::from(&name));
        }
        ui.password_quality().set_value(0.0);
        ui.stack().set_visible_child(&ui.init_page());
        ui.init_button().show();
        ui.init_button().grab_default();
    }
}

fn init_button_clicked(ui: Rc<builder::NewBackup>) {
    let encrypted =
        ui.encryption().get_visible_child() != Some(ui.unencrypted().upcast::<gtk::Widget>());

    if encrypted && ui.password().get_text() != ui.password_confirm().get_text() {
        ui::utils::dialog_error(gettext("Entered passwords do not match. Please try again."));
        return;
    }

    let mut config = if ui.init_location().get_visible_child()
        == Some(ui.init_local().upcast::<gtk::Widget>())
    {
        let mut path = std::path::PathBuf::new();

        if let Some(init_path) = ui.init_path().get_filename() {
            path.push(init_path);
        } else {
            ui::utils::dialog_error(gettext("You have to select a repository location."));
            return;
        }

        path.push(ui.init_dir().get_text().unwrap().as_str());
        trace!("Init repo at {:?}", &path);

        BackupConfig::new_from_path(&path)
    } else {
        let url = ui.init_url().get_text().unwrap().to_string();
        if url.is_empty() {
            ui::utils::dialog_error(gettext("You have to enter a repository location."));
            return;
        }
        BackupConfig::new_from_uri(url)
    };

    config.encrypted = encrypted;
    let mut borg = borg::Borg::new(config.clone());

    if encrypted {
        let password = Zeroizing::new(ui.password().get_text().unwrap().as_bytes().to_vec());

        if ui.password_store().get_active() {
            ui::utils::dialog_catch_err(
                ui::utils::secret_service_set_password(&config, &password),
                gettext("Failed to store password."),
            );
        }
        borg.set_password(password);
    }

    main_pending::show(&gettext("Initializing new backup respository …"));
    ui.new_backup().hide();

    ui::utils::async_react(
        "borg::init",
        move || borg.init(),
        enclose!((config, ui) move |result| {
            if ui::utils::dialog_catch_err(result, gettext("Failed to initialize repository")) {
                main_pending::back();
                ui.new_backup().show();
                return;
            }

            insert_backup_config(config.clone());

            ui.new_backup().close();
        }),
    );
}

fn init_repo_password_changed(ui: &builder::NewBackup) {
    let password = ui.password().get_text().unwrap_or_else(|| "".into());
    let score = if let Ok(pw_check) = zxcvbn::zxcvbn(&password, &[]) {
        if pw_check.score() > 3 {
            let n = pw_check.guesses_log10();
            if (12.0..13.0).contains(&n) {
                5
            } else if (13.0..14.0).contains(&n) {
                6
            } else if n > 14.0 {
                7
            } else {
                4
            }
        } else {
            pw_check.score()
        }
    } else {
        0
    };

    ui.password_quality().set_value(score.into());
}

fn refresh(ui: &ui::builder::NewBackup) {
    debug!("Refreshing list of existing repos");
    let monitor = gio::VolumeMonitor::get();

    ui::utils::clear(&ui.add_repo_list());
    ui::utils::clear(&ui.init_repo_list());

    for mount in monitor.get_mounts() {
        if let Some(mount_point) = mount.get_root().as_ref().and_then(gio::File::get_path) {
            add_mount(&ui.init_repo_list(), &mount, Some(&mount_point));
            if let Ok(dirs) = mount_point.read_dir() {
                for dir in dirs {
                    if let Ok(path) = dir {
                        if is_backup_repo(&path.path()) {
                            add_mount(&ui.add_repo_list(), &mount, Some(&path.path()));
                        }
                    }
                }
            }
        }

        ui.add_repo_list().show_all();
        ui.init_repo_list().show_all();
    }

    debug!("List of existing repos refreshed");
}

fn add_mount(list: &gtk::ListBox, mount: &gio::Mount, repo: Option<&std::path::Path>) {
    let drive = mount.get_drive();

    let name = repo.map(std::path::Path::to_string_lossy);
    let (_, horizontal_box) =
        ui::utils::add_list_box_row(list, name.as_ref().map(std::borrow::Borrow::borrow), 0);

    if let Some(icon) = drive.as_ref().and_then(gio::Drive::get_icon) {
        let img = gtk::Image::new_from_gicon(&icon, gtk::IconSize::Dialog);
        horizontal_box.add(&img);
    }

    let mut label1: String = mount.get_name().map(Into::into).unwrap_or_default();

    let mut label2: String = drive
        .as_ref()
        .and_then(gio::Drive::get_name)
        .map(Into::into)
        .unwrap_or_default();

    if let Some(root) = mount.get_root() {
        if let Some((fs_size, fs_free)) = ui::utils::fs_usage(&root) {
            label2.push_str(&gettext!(
                ", {free} of {size} available",
                free = ui::utils::hsized(fs_free, 0),
                size = ui::utils::hsized(fs_size, 0)
            ));
        }

        if let Some(mount_path) = root.get_path() {
            if let Some(repo_path) = repo {
                if let Ok(suffix) = repo_path.strip_prefix(mount_path) {
                    if !suffix.to_string_lossy().is_empty() {
                        label1.push_str(&format!(" / {}", suffix.to_string_lossy()));
                    }
                }
            }
        }
    }

    let (vertical_box, _, _) =
        ui::utils::list_vertical_box(Some(label1.as_str()), Some(label2.as_str()));
    horizontal_box.add(&vertical_box);
}

fn add_repo_config_local(repo: &std::path::Path, ui: Rc<builder::NewBackup>) {
    let config = BackupConfig::new_from_path(repo);
    insert_backup_config_encryption_unknown(config, ui);
}

fn add_repo_config_remote(uri: String, ui: Rc<builder::NewBackup>) {
    let config = BackupConfig::new_from_uri(uri);
    insert_backup_config_encryption_unknown(config, ui);
}

fn insert_backup_config_encryption_unknown(
    mut config: shared::BackupConfig,
    ui: Rc<builder::NewBackup>,
) {
    let mut borg = borg::Borg::new(config.clone());
    borg.set_password(Zeroizing::new(vec![]));
    config.encrypted = borg.peak().is_err();
    insert_backup_config_password_unknown(config, ui);
}

fn insert_backup_config_password_unknown(config: shared::BackupConfig, ui: Rc<builder::NewBackup>) {
    ui.new_backup().hide();
    let x = config.clone();
    ui::utils::Async::borg(
        "borg::peak",
        borg::Borg::new(x),
        |borg| borg.peak(),
        move |result| match result {
            Ok(()) => {
                insert_backup_config(config.clone());
                ui.new_backup().close();
            }
            Err(borg_err) => {
                debug!("This repo config is not working");
                ui::utils::dialog_error(gettext!(
                    "There was an error with the specified repository:\n\n{}",
                    borg_err
                ));
                ui::utils::dialog_catch_err(
                    ui::utils::secret_service_delete_passwords(&config),
                    "Failed to remove potentially remaining passwords from key storage.",
                );
                ui.new_backup().show();
                main_pending::back();
            }
        },
    )
}

fn insert_backup_config(config: shared::BackupConfig) {
    let uuid = config.id.clone();
    SETTINGS.update(move |s| {
        s.backups.insert(config.id.clone(), config.clone());
    });

    ui::write_config();
    ui::overview::refresh();
    ui::detail::view_backup_conf(&uuid);
}

/// Checks if a directory is most likely a borg repository. Performed checks are
///
/// - `data/` exists and is a directory
/// - `config` exists and contains the string "[repository]"
pub fn is_backup_repo(path: &std::path::Path) -> bool {
    trace!("Checking path if it is a repo '{}'", &path.display());
    if let Ok(data) = std::fs::File::open(path.join("data")).and_then(|x| x.metadata()) {
        if data.is_dir() {
            if let Ok(mut cfg) = std::fs::File::open(path.join("config")) {
                if let Ok(metadata) = cfg.metadata() {
                    if metadata.len() < 1024 * 1024 {
                        let mut content = String::new();
                        #[allow(unused_must_use)]
                        {
                            cfg.read_to_string(&mut content);
                        }
                        return content.contains("[repository]");
                    }
                }
            }
        }
    };

    false
}
