use std::{convert::Into, io::Read, rc::Rc};

use gio::prelude::*;
use gtk::prelude::*;
use libhandy::prelude::*;
use zeroize::Zeroizing;

use crate::borg;
use crate::borg::prelude::*;
use crate::shared;
use crate::shared::*;
use crate::ui;
use crate::ui::builder;
use crate::ui::globals::*;
use crate::ui::prelude::*;
use ui::page_pending;

pub fn new_backup() {
    let ui = Rc::new(ui::builder::DialogAddConfig::new());
    load_available_mounts_and_repos(ui.clone());
    ui.password_quality()
        .add_offset_value(&gtk::LEVEL_BAR_OFFSET_LOW, 7.0);
    ui.password_quality()
        .add_offset_value(&gtk::LEVEL_BAR_OFFSET_HIGH, 5.0);
    ui.password_quality()
        .add_offset_value(&gtk::LEVEL_BAR_OFFSET_FULL, 3.0);

    ui.dialog_vbox().set_border_width(0);

    ui.init_local_row().set_activatable(true);
    ui.init_remote_row().set_activatable(true);
    ui.add_local_row().set_activatable(true);
    ui.add_remote_row().set_activatable(true);

    ui.new_backup().set_transient_for(Some(&main_ui().window()));

    let dialog = ui.new_backup();
    ui.cancel_button().connect_clicked(move |_| {
        dialog.close();
        dialog.hide();
    });

    ui.add_repo_list().connect_row_activated(
        enclose!((ui) move |_, row| on_add_repo_list_activated(row, ui.clone())),
    );

    ui.init_repo_list()
        .connect_row_activated(enclose!((ui) move |_, row| on_init_repo_list_activated(row, &ui)));

    ui.password()
        .connect_changed(enclose!((ui) move |_| on_init_repo_password_changed(&ui)));

    ui.add_button()
        .connect_clicked(enclose!((ui) move |_| on_add_button_clicked(ui.clone())));

    ui.init_button()
        .connect_clicked(enclose!((ui) move |_| on_init_button_clicked(ui.clone())));

    // refresh ui on mount events
    let monitor = gio::VolumeMonitor::get();

    monitor.connect_mount_added(enclose!((ui) move |_, mount| {
        debug!("Mount added");
        load_mount(ui.clone(), mount.clone());
    }));

    monitor.connect_mount_removed(enclose!((ui) move |_, mount| {
        debug!("Mount removed");
        remove_mount(&ui.add_repo_list(), mount.get_root().unwrap().get_uri());
        remove_mount(
            &ui.init_repo_list(),
            mount.get_root().unwrap().get_uri(),
        );
    }));

    ui.new_backup().show_all();
}

fn load_available_mounts_and_repos(ui: Rc<builder::DialogAddConfig>) {
    debug!("Refreshing list of existing repos");
    let monitor = gio::VolumeMonitor::get();

    ui::utils::clear(&ui.add_repo_list());
    ui::utils::clear(&ui.init_repo_list());

    for mount in monitor.get_mounts() {
        load_mount(ui.clone(), mount);
    }

    debug!("List of existing repos refreshed");
}

fn load_mount(ui: Rc<builder::DialogAddConfig>, mount: gio::Mount) {
    if let Some(mount_point) = mount.get_root().unwrap().get_path() {
        add_mount(&ui.init_repo_list(), &mount, None);
        ui::utils::async_react(
            "check_mount_for_repos",
            move || {
                let mut paths = Vec::new();
                if let Ok(dirs) = mount_point.read_dir() {
                    for dir in dirs {
                        if let Ok(path) = dir {
                            if is_backup_repo(&path.path()) {
                                paths.push(path.path());
                            }
                        }
                    }
                }
                paths
            },
            enclose!((ui) move |paths: Result<Vec<std::path::PathBuf>, _>| {
            match paths {
            Err(err) => ui::utils::show_error(gettext("Failed to list existing repositories"), err),
            Ok(paths) =>
                for path in paths {
                    trace!("Adding repo to ui '{:?}'", path);
                    add_mount(&ui.add_repo_list(), &mount, Some(&path));
                }
            }}),
        );
    }
}

fn on_add_repo_list_activated(row: &gtk::ListBoxRow, ui: Rc<builder::DialogAddConfig>) {
    let name = row.get_widget_name();
    if name == "-add-local" {
        add_local(ui);
    } else if name == "-add-remote" {
        ui.stack().set_visible_child(&ui.new_page());
        ui.location_stack().set_visible_child(&ui.location_remote());
        ui.button_stack().set_visible_child(&ui.add_button());
        ui.encryption_box().hide();
        ui.add_button().show();
        ui.add_button().grab_default();
    } else {
        let file = gio::File::new_for_uri(&name);
        add_repo_config_local(&file, ui);
    }
}

fn add_local(ui: Rc<builder::DialogAddConfig>) {
    ui.new_backup().hide();

    if let Some(file) = ui::utils::folder_chooser_dialog(&gettext("Select existing repository")) {
        ui::page_pending::show(&gettext("Loading backup repository"));
        if file
            .get_path()
            .as_deref()
            .map(is_backup_repo)
            .unwrap_or_default()
        {
            add_repo_config_local(&file, ui);
        } else {
            ui::utils::dialog_error(gettext(
                "The selected directory is not a valid backup repository",
            ));
            ui::page_pending::back();
            ui.new_backup().show();
        }
    } else {
        ui.new_backup().show();
    }
}

fn on_add_button_clicked(ui: Rc<builder::DialogAddConfig>) {
    glib::MainContext::default().spawn_local(on_add_button_clicked_future(ui));
}

async fn on_add_button_clicked_future(ui: Rc<builder::DialogAddConfig>) {
    page_pending::show(&gettext("Loading backup repository"));
    ui.new_backup().hide();

    let url = ui.location_url().get_text();
    let file = gio::File::new_for_uri(&url);
    debug!("Add existing URI '{:?}'", file.get_path());

    if url.get(..6) == Some("ssh://") {
        add_repo_config_remote(&url, ui);
    } else if file.get_uri_scheme() == "file"
        || mount_fuse_and_config(&file, ui.clone(), false)
            .await
            .is_ok()
    {
        add_repo_config_local(&file, ui);
    }
}

async fn mount_fuse_and_config(
    file: &gio::File,
    ui: Rc<builder::DialogAddConfig>,
    mount_parent: bool,
) -> Result<BackupRepo, ()> {
    if let (Ok(mount), Some(path)) = (
        file.find_enclosing_mount(Some(&gio::Cancellable::new())),
        file.get_path(),
    ) {
        Ok(BackupRepo::new_local_from_mount(
            mount,
            path,
            file.get_uri().to_string(),
        ))
    } else {
        let mount_uri = if mount_parent {
            file.get_parent().as_ref().unwrap_or(&file).get_uri()
        } else {
            file.get_uri()
        };

        if ui::dialog_device_missing::mount_enclosing(&gio::File::new_for_uri(&mount_uri))
            .await
            .is_err()
        {
            ui::page_pending::back();
            ui.new_backup().show();
            return Err(());
        }

        if let (Ok(mount), Some(path)) = (
            file.find_enclosing_mount(Some(&gio::Cancellable::new())),
            file.get_path(),
        ) {
            Ok(BackupRepo::new_local_from_mount(
                mount,
                path,
                file.get_uri().to_string(),
            ))
        } else {
            ui::utils::show_error(
                gettext("Repository location not mounted"),
                gettext("Mounting succeeded but still unable find enclosing mount"),
            );
            ui::page_pending::back();
            ui.new_backup().show();
            Err(())
        }
    }
}

fn on_init_repo_list_activated(row: &gtk::ListBoxRow, ui: &builder::DialogAddConfig) {
    let name = row.get_widget_name();

    ui.button_stack().set_visible_child(&ui.init_button());
    if name == "-init-remote" {
        ui.location_stack().set_visible_child(&ui.location_remote());
    } else {
        ui.location_stack().set_visible_child(&ui.location_local());
        if name != "-init-local" {
            trace!("Setting {} as init_path", &name);
            ui.init_path().set_current_folder_uri(&name);
        } else {
            ui.init_path().grab_focus();
        }
    }
    show_init(ui);
}

fn show_init(ui: &builder::DialogAddConfig) {
    ui.init_dir().set_text(&format!(
        "backup-{}-{}",
        glib::get_host_name()
            .map(|x| x.to_string())
            .unwrap_or_default(),
        glib::get_user_name()
            .and_then(|x| x.into_string().ok())
            .unwrap_or_default()
    ));
    ui.password_quality().set_value(0.0);
    ui.stack().set_visible_child(&ui.new_page());
    ui.init_button().show();
    ui.init_button().grab_default();
}

fn on_init_button_clicked(ui: Rc<builder::DialogAddConfig>) {
    glib::MainContext::default().spawn_local(on_init_button_clicked_future(ui));
}

async fn on_init_button_clicked_future(ui: Rc<builder::DialogAddConfig>) {
    let encrypted =
        ui.encryption().get_visible_child() != Some(ui.unencrypted().upcast::<gtk::Widget>());

    // TODO: Add string for empty password
    if encrypted
        && (ui.password().get_text() != ui.password_confirm().get_text()
            || ui.password().get_text().is_empty())
    {
        ui::utils::show_error(
            gettext("Entered passwords do not match"),
            gettext("Please try again"),
        );
        return;
    }

    let repo_opt = if ui.location_stack().get_visible_child()
        == Some(ui.location_local().upcast::<gtk::Widget>())
    {
        if let Some(file) = ui
            .init_path()
            .get_file()
            .map(|x| x.get_child(ui.init_dir().get_text().as_str()).unwrap())
        {
            BackupRepo::new_local_for_file(&file)
        } else {
            None
        }
    } else {
        let url = ui.location_url().get_text().to_string();
        let file = gio::File::new_for_uri(&ui.location_url().get_text());

        if url.is_empty() {
            None
        } else if url.get(..6) == Some("ssh://") {
            Some(BackupRepo::new_remote(url))
        } else if file.get_uri_scheme() == "file" {
            BackupRepo::new_local_for_file(&file)
        } else {
            mount_fuse_and_config(&gio::File::new_for_uri(&url), ui.clone(), true)
                .await
                .ok()
        }
    };

    let mut repo = {
        if let Some(repo) = repo_opt {
            repo
        } else {
            ui::utils::dialog_error(gettext("You have to enter a repository location"));
            return;
        }
    };

    if let Ok(args) = get_command_line_args(&ui) {
        repo.set_settings(Some(BackupSettings {
            command_line_args: Some(args),
        }));
    } else {
        ui::utils::dialog_error(gettext("Invalid additional command line arguments"));
        return;
    }

    page_pending::show(&gettext("Creating backup repository"));
    ui.new_backup().hide();

    let mut borg = borg::BorgOnlyRepo::new(repo.clone());
    let password = Zeroizing::new(ui.password().get_text().as_bytes().to_vec());
    if encrypted {
        borg.set_password(password.clone());
    }

    ui::utils::async_react(
        "borg::init",
        move || borg.init(),
        enclose!((repo, ui, password) move |result: Result<Result<borg::List, _>,_>|

        match result.unwrap_or(Err(shared::BorgErr::ThreadPanicked)) {

            Err(err) => {
                ui::utils::show_error(&gettext("Failed to initialize repository"), &err);
                page_pending::back();
                ui.new_backup().show();
            }
            Ok(info) => {
                let config = shared::BackupConfig::new(repo.clone(), info, encrypted);

                insert_backup_config(config.clone());
                if encrypted && ui.password_store().get_active() {
                    ui::utils::dialog_catch_err(
                        ui::utils::secret_service_set_password(&config, &password),
                        gettext("Failed to store password."),
                    );
                }
                ui::page_detail::view_backup_conf(&config.id);

                ui.new_backup().close();
            }
        }),
    );
}

fn get_command_line_args(ui: &builder::DialogAddConfig) -> Result<Vec<String>, ()> {
    if let Ok(args) = shell_words::split(
        &ui.command_line_args()
            .get_buffer()
            .and_then(|buffer| {
                let (start, end) = buffer.get_bounds();
                buffer.get_text(&start, &end, false).map(|x| x.to_string())
            })
            .unwrap_or_default(),
    ) {
        Ok(args)
    } else {
        ui::utils::show_error(
            gettext("Additional command line arguments invalid"),
            gettext("Please check for missing closing quotes"),
        );
        Err(())
    }
}

fn on_init_repo_password_changed(ui: &builder::DialogAddConfig) {
    let password = ui.password().get_text();
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

fn remove_mount(list: &gtk::ListBox, root: glib::GString) {
    for list_row in list.get_children() {
        if list_row.get_widget_name() == root {
            list.remove(&list_row);
        }
    }
}

fn add_mount(list: &gtk::ListBox, mount: &gio::Mount, repo: Option<&std::path::Path>) {
    let row = ui::utils::new_action_row_with_gicon(mount.get_icon().as_ref());
    list.add(&row);

    row.set_widget_name(&mount.get_root().unwrap().get_uri());

    let mut label1 = mount.get_name().map(|x| x.to_string()).unwrap_or_default();

    let mut label2: String = mount
        .get_drive()
        .as_ref()
        .and_then(gio::Drive::get_name)
        .map(Into::into)
        .unwrap_or_else(|| mount.get_root().unwrap().get_uri().to_string());

    if let Some((fs_size, fs_free)) = ui::utils::fs_usage(&mount.get_root().unwrap()) {
        label2.push_str(&gettextf(
            " – {} of {} available",
            &[
                &glib::format_size(fs_free).unwrap(),
                &glib::format_size(fs_size).unwrap(),
            ],
        ));
    }

    if let Some(mount_path) = mount.get_root().unwrap().get_path() {
        if let Some(repo_path) = repo {
            row.set_widget_name(&gio::File::new_for_path(repo_path).get_uri());
            if let Ok(suffix) = repo_path.strip_prefix(mount_path) {
                if !suffix.to_string_lossy().is_empty() {
                    label1.push_str(&format!(" / {}", suffix.to_string_lossy()));
                }
            }
        }
    }

    row.set_title(Some(label1.as_str()));
    row.set_subtitle(Some(label2.as_str()));

    list.show_all();
}

fn add_repo_config_local(file: &gio::File, ui: Rc<builder::DialogAddConfig>) {
    if let Some(repo) = BackupRepo::new_local_for_file(file) {
        insert_backup_config_encryption_unknown(repo, ui);
    } else {
        ui::utils::dialog_error(gettext("Unexpected error with repository"));
        ui.new_backup().show();
        page_pending::back();
    }
}

fn add_repo_config_remote(uri: &str, ui: Rc<builder::DialogAddConfig>) {
    let mut repo = BackupRepo::new_remote(uri.to_string());

    if let Ok(args) = get_command_line_args(&ui) {
        repo.set_settings(Some(BackupSettings {
            command_line_args: Some(args),
        }));
    } else {
        return;
    }

    insert_backup_config_encryption_unknown(repo, ui);
}

fn insert_backup_config_encryption_unknown(
    repo: shared::BackupRepo,
    ui: Rc<builder::DialogAddConfig>,
) {
    ui.new_backup().hide();

    ui::utils::Async::borg_only_repo_suggest_store(
        "borg::peek",
        borg::BorgOnlyRepo::new(repo.clone()),
        |borg| borg.peek(),
        move |result| match result {
            Ok((info, pw_data)) => {
                let encrypted = pw_data
                    .clone()
                    .map(|(password, _)| !password.is_empty())
                    .unwrap_or_default();
                let config = shared::BackupConfig::new(repo.clone(), info, encrypted);
                insert_backup_config(config.clone());
                ui::utils::store_password(&config, &pw_data);
                ui::page_detail::view_backup_conf(&config.id);
                ui.new_backup().close();
            }
            Err(borg_err) => {
                debug!("This repo config is not working");
                ui::utils::show_error(
                    gettext("There was an error with the specified repository"),
                    borg_err,
                );
                ui.new_backup().show();
                page_pending::back();
            }
        },
    )
}

fn insert_backup_config(config: shared::BackupConfig) {
    SETTINGS.update(move |s| {
        s.backups.insert(config.id.clone(), config.clone());
    });

    ui::write_config();
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
