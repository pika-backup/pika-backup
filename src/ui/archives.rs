use std::collections::BTreeMap;

use arc_swap::ArcSwap;
use chrono::prelude::*;
use gio::prelude::*;
use gtk::prelude::*;
use once_cell::sync::Lazy;

use crate::borg;
use crate::shared::*;
use crate::ui;
use crate::ui::globals::*;
use crate::ui::prelude::*;
use crate::ui::utils::BackupMap;

static ARCHIVES: Lazy<ArcSwap<BTreeMap<String, borg::ListArchive>>> = Lazy::new(Default::default);

fn with_archive<T, F>(f: F) -> T
where
    F: Fn(&borg::ListArchive) -> T,
{
    let row = &main_ui().archive_list().get_selected_rows()[0];
    let archive_id = row.get_widget_name().unwrap().to_string();

    f(ARCHIVES.load().get(&archive_id).as_ref().unwrap())
}

pub fn init() {
    main_ui().archive_list().connect_row_activated(|_, row| {
        with_archive(|archive| {
            main_ui()
                .archive_start()
                .set_text(&archive.start.to_locale());
            main_ui().archive_end().set_text(&archive.end.to_locale());
            main_ui().archive_name().set_text(&archive.name);
            main_ui().archive_hostname().set_text(&archive.hostname);
            main_ui().archive_username().set_text(&archive.username);

            if archive.comment.is_empty() {
                main_ui().archive_comment().hide();
            } else {
                main_ui().archive_comment().set_text(&archive.comment);
                main_ui().archive_comment().show();
            }
        });
        main_ui().archive_popover().set_relative_to(Some(row));
        main_ui().archive_popover().popup();
    });

    main_ui()
        .browse_archive()
        .connect_clicked(|_| on_browse_archive());
}

fn on_browse_archive() {
    main_ui().pending_menu().show();
    main_ui().archive_popover().popdown();

    let backup = SETTINGS.load().backups.get_active().unwrap().clone();
    let backup_mounted = ACTIVE_MOUNTS.load().contains(&backup.id);

    with_archive(|archive| {
        let backup = backup.clone();
        let mut path = borg::Borg::new(backup.clone()).get_mount_point();
        path.push(archive.name.clone());

        let open_archive = || {
            let none: Option<&gio::AppLaunchContext> = None;
            ui::utils::async_react(
                "open_archive",
                move || find_first_populated_dir(&path),
                move |path| {
                    let uri = gio::File::new_for_path(&path).get_uri();

                    // only open if app isn't closing in this moment
                    if !**IS_SHUTDOWN.load() {
                        ui::utils::dialog_catch_err(
                            gio::AppInfo::launch_default_for_uri(&uri, none),
                            gettext("Failed to open archive."),
                        );
                    }

                    main_ui().pending_menu().hide();
                },
            )
        };

        if backup_mounted {
            open_archive();
        } else {
            ACTIVE_MOUNTS.update(|mounts| {
                mounts.insert(backup.id.clone());
            });
            ui::utils::Async::borg(
                "mount_archive",
                borg::Borg::new(backup.clone()),
                move |borg| borg.mount(),
                move |mount| {
                    let open_archive = open_archive.clone();
                    if mount.is_ok() {
                        trace!("Mount successful");
                        open_archive();
                    } else {
                        ACTIVE_MOUNTS.update(|mounts| {
                            mounts.remove(&backup.id.clone());
                        });
                    }

                    ui::utils::dialog_catch_err(
                        mount,
                        gettext("Failed to make archives available for browsing."),
                    );
                },
            );
        }
    });
}

fn show_archives(archive_list: Result<Vec<borg::ListArchive>, BorgErr>) {
    ARCHIVES.update(|archives| archives.clear());

    match archive_list {
        Ok(archive_list) => {
            for archive in archive_list {
                let id = archive.name.clone();
                let (_row, horizontal_box) =
                    ui::utils::add_list_box_row(&main_ui().archive_list(), Some(&id), 0);

                let text = gettext!(
                    "{hostname}, {username}, about {ago}",
                    ago = (archive.end - Local::now().naive_local()).humanize(),
                    hostname = archive.hostname,
                    username = archive.username
                );
                horizontal_box.add(&gtk::Label::new(Some(text.as_str())));
                ARCHIVES.update(move |a| {
                    a.insert(id.clone(), archive.clone());
                });
            }
        }
        err @ Err(_) => {
            ui::utils::dialog_catch_err(
                err,
                gettext("An error occured while retriving the list of archives."),
            );
        }
    }

    main_ui()
        .archive_list()
        .set_placeholder(None::<&gtk::Widget>);
    main_ui().archive_list().show_all();
}

pub fn show() {
    let backup = SETTINGS.load().backups.get_active().unwrap().clone();
    ui::utils::clear(&main_ui().archive_list());
    main_ui().main_stack().set_visible_child_name("archives");

    let label = gtk::Spinner::new();
    main_ui().archive_list().set_placeholder(Some(&label));
    label.start();
    label.show();

    ui::utils::Async::borg(
        "list_archives",
        borg::Borg::new(backup),
        |borg| borg.list(),
        show_archives,
    );
}

fn find_first_populated_dir(dir: &std::path::Path) -> std::path::PathBuf {
    if let Ok(mut dir_iter) = dir.read_dir() {
        if let Some(Ok(new_dir)) = dir_iter.next() {
            if new_dir.path().is_dir() && dir_iter.next().is_none() {
                return find_first_populated_dir(&new_dir.path());
            }
        }
    }

    dir.to_path_buf()
}
