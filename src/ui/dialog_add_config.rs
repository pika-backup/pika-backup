mod insert;

use std::{convert::Into, rc::Rc};

use gio::prelude::*;
use gtk::prelude::*;
use libhandy::prelude::*;

use crate::ui;
use crate::ui::builder;
use crate::ui::prelude::*;

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

    ui.dialog().set_transient_for(Some(&main_ui().window()));

    let dialog = ui.dialog();
    ui.cancel_button().connect_clicked(move |_| {
        dialog.close();
        dialog.hide();
    });

    ui.add_repo_list()
        .connect_row_activated(enclose!((ui) move |_, row| {
            insert::on_add_repo_list_activated(Rc::new(row.clone()), ui.clone());
        }));

    ui.init_repo_list()
        .connect_row_activated(enclose!((ui) move |_, row| on_init_repo_list_activated(row, &ui)));

    ui.password()
        .connect_changed(enclose!((ui) move |_| on_init_repo_password_changed(&ui)));

    ui.add_button()
        .connect_clicked(enclose!((ui) move |_| insert::on_add_button_clicked(ui.clone())));

    ui.init_button()
        .connect_clicked(enclose!((ui) move |_| insert::on_init_button_clicked(ui.clone())));

    ui.location_url()
        .connect_icon_press(enclose!((ui) move |_, _, _| on_location_url_help(&ui)));

    // refresh ui on mount events
    let monitor = gio::VolumeMonitor::get();

    monitor.connect_mount_added(enclose!((ui) move |_, mount| {
        debug!("Mount added");
        Handler::new().error_transient_for(ui.dialog())
        .spawn(load_mount(ui.clone(), mount.clone()));
    }));

    monitor.connect_mount_removed(enclose!((ui) move |_, mount| {
        debug!("Mount removed");
        remove_mount(&ui.add_repo_list(), mount.get_root().unwrap().get_uri());
        remove_mount(
            &ui.init_repo_list(),
            mount.get_root().unwrap().get_uri(),
        );
    }));

    ui.dialog().show_all();
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

fn load_available_mounts_and_repos(ui: Rc<builder::DialogAddConfig>) {
    debug!("Refreshing list of existing repos");
    let monitor = gio::VolumeMonitor::get();

    ui::utils::clear(&ui.add_repo_list());
    ui::utils::clear(&ui.init_repo_list());

    for mount in monitor.get_mounts() {
        Handler::new()
            .error_transient_for(ui.dialog())
            .spawn(load_mount(ui.clone(), mount));
    }

    debug!("List of existing repos refreshed");
}

async fn load_mount(ui: Rc<builder::DialogAddConfig>, mount: gio::Mount) -> Result<()> {
    if let Some(mount_point) = mount.get_root().unwrap().get_path() {
        add_mount(&ui.init_repo_list(), &mount, None);
        let paths = ui::utils::spawn_thread("check_mount_for_repos", move || {
            let mut paths = Vec::new();
            if let Ok(dirs) = mount_point.read_dir() {
                for dir in dirs {
                    if let Ok(path) = dir {
                        if ui::utils::is_backup_repo(&path.path()) {
                            paths.push(path.path());
                        }
                    }
                }
            }
            paths
        })
        .await;

        match paths {
            Err(err) => {
                return Err(
                    Message::new(gettext("Failed to list existing repositories."), err).into(),
                );
            }
            Ok(paths) => {
                for path in paths {
                    trace!("Adding repo to ui '{:?}'", path);
                    add_mount(&ui.add_repo_list(), &mount, Some(&path));
                }
            }
        }
    }

    Ok(())
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

fn on_location_url_help(ui: &builder::DialogAddConfig) {
    ui.location_url_help().popup();
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
        label1.push_str(&format!(" – {}", &glib::format_size(fs_size).unwrap()));

        label2.push_str(" – ");
        label2.push_str(&gettextf(
            "Free space: {}",
            &[&glib::format_size(fs_free).unwrap()],
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
