mod event;
pub mod folder_button;
mod insert;

use std::{convert::Into, rc::Rc};

use adw::prelude::*;

use crate::ui;
use crate::ui::builder;
use crate::ui::prelude::*;

thread_local! {
    static VOLUME_MONITOR: gio::VolumeMonitor = gio::VolumeMonitor::get();
}

pub fn new_backup() {
    let ui = Rc::new(ui::builder::DialogAddConfig::new());
    load_available_mounts_and_repos(ui.clone());
    ui.password_quality()
        .add_offset_value(&gtk::LEVEL_BAR_OFFSET_LOW, 7.0);
    ui.password_quality()
        .add_offset_value(&gtk::LEVEL_BAR_OFFSET_HIGH, 5.0);
    ui.password_quality()
        .add_offset_value(&gtk::LEVEL_BAR_OFFSET_FULL, 3.0);

    ui.init_local_row().set_activatable(true);
    ui.init_remote_row().set_activatable(true);
    ui.add_local_row().set_activatable(true);
    ui.add_remote_row().set_activatable(true);

    ui.dialog().set_transient_for(Some(&main_ui().window()));

    ui.back_to_overview()
        .connect_clicked(enclose!((ui) move |_| event::back_to_overview(&*ui)));

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
        .connect_icon_press(enclose!((ui) move |_, _| on_location_url_help(&ui)));

    ui.init_path()
        .connect_folder_change(enclose!((ui) move || on_path_change(&ui)));

    // refresh ui on mount events

    // TODO: Memory leak: Old monitors handlers for closed windows remain
    VOLUME_MONITOR.with(|monitor| {
        monitor.connect_mount_added(enclose!((ui) move |_, mount| {
            debug!("Mount added");
            Handler::new().error_transient_for(ui.dialog())
            .spawn(load_mount(ui.clone(), mount.clone()));
        }));

        monitor.connect_mount_removed(enclose!((ui) move |_, mount| {
            debug!("Mount removed");
            remove_mount(&ui.add_repo_list(), mount.root().uri());
            remove_mount(
                &ui.init_repo_list(),
                mount.root().uri(),
            );
        }));
    });

    ui.dialog().present();
}

fn on_path_change(ui: &builder::DialogAddConfig) {
    if let Some(path) = ui.init_path().file().and_then(|x| x.path()) {
        let mount_entry = gio::UnixMountEntry::for_file_path(&path);
        if let Some(fs) = mount_entry.0.map(|x| x.fs_type()) {
            debug!("Selected filesystem type {}", fs);
            ui.non_journaling_warning()
                .set_visible(crate::NON_JOURNALING_FILESYSTEMS.iter().any(|x| x == &fs));
        } else {
            ui.non_journaling_warning().hide();
        }
    } else {
        ui.non_journaling_warning().hide();
    }
}

fn on_init_repo_list_activated(row: &gtk::ListBoxRow, ui: &builder::DialogAddConfig) {
    let name = row.widget_name();

    ui.button_stack().set_visible_child(&ui.init_button());
    if name == "-init-remote" {
        ui.location_stack().set_visible_child(&ui.location_remote());
    } else {
        ui.location_stack().set_visible_child(&ui.location_local());
        if name != "-init-local" {
            trace!("Setting {} as init_path", &name);
            ui.init_path()
                .set_property("file", gio::File::for_uri(&name));
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

    for mount in monitor.mounts() {
        Handler::new()
            .error_transient_for(ui.dialog())
            .spawn(load_mount(ui.clone(), mount));
    }

    debug!("List of existing repos refreshed");
}

async fn load_mount(ui: Rc<builder::DialogAddConfig>, mount: gio::Mount) -> Result<()> {
    if let Some(mount_point) = mount.root().path() {
        add_mount(&ui.init_repo_list(), &mount, None).await;
        let paths = ui::utils::spawn_thread("check_mount_for_repos", move || {
            let mut paths = Vec::new();
            if let Ok(dirs) = mount_point.read_dir() {
                for path in dirs.flatten() {
                    if ui::utils::is_backup_repo(&path.path()) {
                        paths.push(path.path());
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
                    add_mount(&ui.add_repo_list(), &mount, Some(&path)).await;
                }
            }
        }
    }

    Ok(())
}

fn show_init(ui: &builder::DialogAddConfig) {
    ui.init_dir().set_text(&format!(
        "backup-{}-{}",
        glib::host_name(),
        glib::user_name().to_string_lossy()
    ));
    ui.password_quality().set_value(0.0);
    on_path_change(ui);
    ui.leaflet().set_visible_child(&ui.page_detail());
    ui.init_button().show();
    ui.dialog().set_default_widget(Some(&ui.init_button()));
}

fn on_init_repo_password_changed(ui: &builder::DialogAddConfig) {
    let password = ui.password().text();
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
    let mut i = 0;
    while let Some(list_row) = list.row_at_index(i) {
        if list_row.widget_name() == root {
            list.remove(&list_row);
            break;
        }
        i += 1
    }
}

async fn add_mount(list: &gtk::ListBox, mount: &gio::Mount, repo: Option<&std::path::Path>) {
    let row = ui::utils::new_action_row_with_gicon(Some(mount.icon().as_ref()));
    list.append(&row);

    row.set_widget_name(&mount.root().uri());

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

    row.set_title(&label1);
    row.set_subtitle(&label2);
}
