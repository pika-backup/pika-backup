pub mod add_task;
mod display;
mod event;
pub mod folder_button;
mod insert;
mod remote_location;

use std::convert::Into;

use adw::prelude::*;

use crate::ui;
use crate::ui::prelude::*;
use insert::execute;
use ui::builder::DialogSetup;

pub fn show() {
    let ui = DialogSetup::new();

    ui.dialog().set_transient_for(Some(&main_ui().window()));

    // Page Overview

    ui.init_local_row()
        .connect_activated(clone!(@weak  ui => move |_| event::show_init_local(&ui)));

    ui.init_remote_row()
        .connect_activated(clone!(@weak ui => move |_| event::show_init_remote(&ui)));

    ui.add_local_row()
        .connect_activated(clone!(@weak ui =>  move |_| event::show_add_local(&ui)));

    ui.add_remote_row()
        .connect_activated(clone!(@weak ui => move |_| event::show_add_remote(&ui)));

    // Page Detail

    load_available_mounts_and_repos(&ui);

    ui.password_quality()
        .add_offset_value(&gtk::LEVEL_BAR_OFFSET_LOW, 7.0);
    ui.password_quality()
        .add_offset_value(&gtk::LEVEL_BAR_OFFSET_HIGH, 5.0);
    ui.password_quality()
        .add_offset_value(&gtk::LEVEL_BAR_OFFSET_FULL, 3.0);

    ui.leaflet()
        .connect_visible_child_notify(clone!(@weak ui => move |_| event::leaflet_change(&ui)));

    ui.back_to_overview()
        .connect_clicked(clone!(@weak ui => move |_| event::back_to_overview(&ui)));

    ui.password()
        .connect_changed(clone!(@weak ui => move |_| event::password_changed(&ui)));

    ui.add_button().connect_clicked(
        clone!(@weak ui => move |_| execute(event::add_remote(ui.clone()), ui.dialog())),
    );

    ui.init_button()
        .connect_clicked(clone!(@weak ui => move |_| event::init_repo(&ui)));

    ui.init_path()
        .connect_folder_change(clone!(@weak ui => move || event::path_change(&ui)));

    // Page Password

    let run = clone!(@weak ui =>
        move |x| insert::execute(x, ui.dialog())
    );

    ui.page_password_continue()
        .connect_clicked(clone!(@weak ui => move |_| run(event::page_password_continue(ui))));

    ui.page_password()
        .connect_visible_child_notify(clone!(@weak ui => move |_| event::leaflet_change(&ui)));

    ui.pending_spinner().connect_map(|s| s.start());
    ui.pending_spinner().connect_unmap(|s| s.stop());

    ui.transfer_pending_spinner().connect_map(|s| s.start());
    ui.transfer_pending_spinner().connect_unmap(|s| s.stop());

    // refresh ui on mount events

    let volume_monitor = gio::VolumeMonitor::get();

    volume_monitor.connect_mount_added(clone!(@weak ui => move |_, mount| {
        debug!("Mount added");
        Handler::new().error_transient_for(ui.dialog())
        .spawn(load_mount(ui, mount.clone()));
    }));

    volume_monitor.connect_mount_removed(clone!(@weak ui => move |_, mount| {
        debug!("Mount removed");
        remove_mount(&ui.add_repo_list(), mount.root().uri());
        remove_mount(
            &ui.init_repo_list(),
            mount.root().uri(),
        );
    }));

    let dialog = ui.dialog();

    // ensure lifetime until window closes
    let mutex = std::sync::Mutex::new(Some((ui, volume_monitor)));
    dialog.connect_close_request(move |_| {
        *mutex.lock().unwrap() = None;
        gtk::Inhibit(false)
    });

    dialog.connect_destroy(|_| {
        debug!("Destroy dialog");
    });

    dialog.present();
}

fn load_available_mounts_and_repos(ui: &DialogSetup) {
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

async fn load_mount(ui: DialogSetup, mount: gio::Mount) -> Result<()> {
    if let Some(mount_point) = mount.root().path() {
        display::add_mount(
            &ui.init_repo_list(),
            &mount,
            None,
            clone!(@weak ui, @strong mount_point => move || {
                display::show_init_local(&ui, Some(&mount_point))
            }),
        )
        .await;
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
                    display::add_mount(
                        &ui.add_repo_list(),
                        &mount,
                        Some(&path),
                        clone!(@weak ui, @strong path => move || {
                            event::add_local(&ui, Some(&path))
                        }),
                    )
                    .await;
                }
            }
        }
    }

    Ok(())
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
