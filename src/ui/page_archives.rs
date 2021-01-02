use std::collections::{BTreeMap, HashMap};
use std::iter::FromIterator;

use arc_swap::ArcSwap;
use gio::prelude::*;
use gtk::prelude::*;
use libhandy::prelude::*;
use once_cell::sync::Lazy;

use crate::borg;
use crate::borg::prelude::*;
use crate::shared::*;
use crate::ui;
use crate::ui::globals::*;
use crate::ui::utils::BackupMap;
use ui::prelude::*;

static REPO_ARCHIVES: Lazy<ArcSwap<BTreeMap<String, RepoArchives>>> = Lazy::new(Default::default);

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct RepoArchives {
    archives: Option<HashMap<String, borg::ListArchive>>,
    reloading: bool,
}

impl RepoArchives {
    pub fn archives_sorted_by_date(&self) -> Option<Vec<(String, borg::ListArchive)>> {
        if let Some(archives) = self.archives.clone() {
            let mut vec = Vec::from_iter(archives);
            vec.sort_by(|x, y| y.1.start.cmp(&x.1.start));
            Some(vec)
        } else {
            None
        }
    }
}

pub fn init() {
    main_ui()
        .detail_stack()
        .connect_property_visible_child_notify(|_| {
            if page_is_visible() {
                show();
            }
        });

    main_ui().refresh_archives().connect_clicked(|_| {
        let config = SETTINGS.load().backups.get_active().unwrap().clone();
        ui::dialog_device_missing::main(config.clone(), "", move || {
            refresh_archives_cache(config.clone());
        });
    });

    main_ui()
        .archives_reloading_spinner()
        .connect_map(|s| s.start());
    main_ui()
        .archives_reloading_spinner()
        .connect_unmap(|s| s.stop());
}

pub fn refresh_archives_cache(config: BackupConfig) {
    info!("Refreshing archives cache");

    if Some(true)
        == REPO_ARCHIVES
            .load()
            .get(&config.repo_id)
            .map(|x| x.reloading)
    {
        info!("Aborting archives cache reload because already in progress");
        return;
    } else {
        REPO_ARCHIVES.update(|repos| {
            let mut repo = repos.get(&config.repo_id).cloned().unwrap_or_default();
            repo.reloading = true;
            repos.insert(config.repo_id.clone(), repo);
        });
    }

    update_archives_spinner(config.clone());

    ui::utils::Async::borg(
        "refresh_archives_cache",
        borg::Borg::new(config.clone()),
        |borg| borg.list(100),
        move |result| archives_cache_refreshed(config.clone(), result),
    );
}

fn update_archives_spinner(config: BackupConfig) {
    if Some(&config.repo_id) == SETTINGS.load().backups.get_active().map(|x| &x.repo_id)
        && page_is_visible()
    {
        let reloading = REPO_ARCHIVES
            .load()
            .get(&config.repo_id)
            .map(|x| x.reloading)
            .unwrap_or_default();

        if reloading {
            main_ui()
                .archives_reloading_stack()
                .set_visible_child(&main_ui().archives_reloading_spinner());
        } else {
            main_ui()
                .archives_reloading_stack()
                .set_visible_child(&main_ui().refresh_archives());
        }
    }
}

fn archives_cache_refreshed(config: BackupConfig, result: Result<Vec<borg::ListArchive>, BorgErr>) {
    match result {
        Ok(archives) => {
            let mut repo_archives = REPO_ARCHIVES
                .load()
                .get(&config.repo_id)
                .cloned()
                .unwrap_or_default();
            repo_archives.archives = Some(
                archives
                    .iter()
                    .map(|x| (x.name.clone(), x.clone()))
                    .collect(),
            );
            repo_archives.reloading = false;
            REPO_ARCHIVES.update(enclose!((config, repo_archives) move |repos| {
                repos.insert(config.repo_id.clone(), repo_archives.clone());
            }));
            info!("Archives cache refreshed");
            match std::fs::DirBuilder::new()
                .recursive(true)
                .create(cache_dir())
                .and_then(|_| std::fs::File::create(cache_path(&config.repo_id)))
            {
                Ok(file) => {
                    ui::utils::dialog_catch_err(
                        serde_json::ser::to_writer(&file, &repo_archives),
                        "Failed to save cache.",
                    );
                }
                Err(err) => {
                    ui::utils::show_error("Failed to open cache file.", err);
                }
            }

            display_archives(config);
        }
        Err(err) => {
            REPO_ARCHIVES.update(|repos| {
                let mut repo = repos.get(&config.repo_id).cloned().unwrap_or_default();
                repo.reloading = false;
                repos.insert(config.repo_id.clone(), repo);
            });
            update_archives_spinner(config);
            ui::utils::show_error("Failed to refresh archives cache.", err)
        }
    }
}

fn on_browse_archive(config: BackupConfig, archive_name: String) {
    debug!("Trying to browse an archive");

    main_ui().pending_menu().show();

    let backup_mounted = ACTIVE_MOUNTS.load().contains(&config.id);

    let mut path = borg::Borg::new(config.clone()).get_mount_point();
    path.push(archive_name);

    let open_archive = || {
        ui::utils::async_react(
            "open_archive",
            move || find_first_populated_dir(&path),
            move |result| {
                match result {
                    Err(err) => ui::utils::show_error(gettext("Failed to open archive."), err),
                    Ok(path) => {
                        let uri = gio::File::new_for_path(&path).get_uri();

                        // only open if app isn't closing in this moment
                        if !**IS_SHUTDOWN.load() {
                            let show_folder = || -> Result<(), _> {
                                let conn = dbus::blocking::Connection::new_session()?;
                                let proxy = conn.with_proxy(
                                    "org.freedesktop.FileManager1",
                                    "/org/freedesktop/FileManager1",
                                    std::time::Duration::from_millis(5000),
                                );
                                proxy.method_call(
                                    "org.freedesktop.FileManager1",
                                    "ShowFolders",
                                    (vec![uri.as_str()], ""),
                                )
                            };

                            ui::utils::dialog_catch_err(
                                show_folder(),
                                gettext("Failed to open archive."),
                            );
                        }
                    }
                };

                main_ui().pending_menu().hide();
            },
        )
    };

    if backup_mounted {
        open_archive();
    } else {
        ACTIVE_MOUNTS.update(|mounts| {
            mounts.insert(config.id.clone());
        });
        ui::utils::Async::borg(
            "mount_archive",
            borg::Borg::new(config.clone()),
            move |borg| borg.mount(),
            move |mount| {
                let open_archive = open_archive.clone();
                if mount.is_ok() {
                    trace!("Mount successful");
                    open_archive();
                } else {
                    ACTIVE_MOUNTS.update(|mounts| {
                        mounts.remove(&config.id.clone());
                    });
                }

                ui::utils::dialog_catch_err(
                    mount,
                    gettext("Failed to make archives available for browsing."),
                );
            },
        );
    }
}

fn page_is_visible() -> bool {
    main_ui().detail_stack().get_visible_child()
        == Some(main_ui().page_archives().upcast::<gtk::Widget>())
}

fn display_archives(config: BackupConfig) {
    if Some(&config.repo_id) == SETTINGS.load().backups.get_active().map(|x| &x.repo_id)
        && page_is_visible()
    {
        debug!("Displaying archive list from cache");
        let repo_archives = REPO_ARCHIVES.load();

        ui::utils::clear(&main_ui().archive_list());
        update_archives_spinner(config.clone());
        if let Some(archive_list) = repo_archives
            .get(&config.repo_id)
            .and_then(|x| x.archives_sorted_by_date())
        {
            for (id, archive) in archive_list {
                let row = libhandy::ExpanderRow::new();
                row.set_subtitle(Some(&format!(
                    "{hostname}, {username}",
                    hostname = archive.hostname,
                    username = archive.username
                )));
                row.set_title(Some(&archive.start.to_locale()));

                let info = |title: String, info: String| -> libhandy::ActionRow {
                    let row = libhandy::ActionRow::new();
                    row.set_title(Some(&title));
                    let label = gtk::Label::new(Some(&info));
                    label.add_css_class("dim-label");
                    label.set_line_wrap(true);
                    label.set_line_wrap_mode(pango::WrapMode::WordChar);
                    row.add(&label);
                    row
                };

                row.add(&info(gettext("Name"), archive.name.clone()));
                row.add(&info(
                    gettext("Duration"),
                    // TODO: Translate durations
                    format!("About {}", (archive.end - archive.start).humanize()),
                ));
                if !archive.comment.is_empty() {
                    row.add(&info(gettext("Comment"), archive.comment.clone()));
                }

                let browse_row = libhandy::ActionRow::new();
                browse_row.set_title(Some(&gettext("Browse saved files")));
                browse_row.set_activatable(true);
                row.add(&browse_row);

                let browse_button =
                    gtk::Image::from_icon_name(Some("go-next-symbolic"), gtk::IconSize::Button);
                browse_row.add(&browse_button);
                browse_row.set_icon_name("folder-open-symbolic");

                browse_row.connect_activated(
                    enclose!((config, id) move |_| on_browse_archive(config.clone(), id.clone())),
                );

                main_ui().archive_list().add(&row);
            }

            main_ui().archive_list().show_all();
        }

        if repo_archives
            .get(&config.repo_id)
            .and_then(|x| x.archives.as_ref())
            .map(|x| !x.is_empty())
            .unwrap_or_default()
        {
            main_ui()
                .archives_stack()
                .set_visible_child(&main_ui().archive_list());
        } else {
            main_ui()
                .archives_stack()
                .set_visible_child(&main_ui().archive_list_placeholder());
        }
    } else {
        debug!("Not displaying archive list because it's not visible");
    }
}

fn cache_dir() -> std::path::PathBuf {
    [
        glib::get_user_cache_dir().unwrap(),
        env!("CARGO_PKG_NAME").into(),
    ]
    .iter()
    .collect()
}

fn cache_path(repo_id: &str) -> std::path::PathBuf {
    [cache_dir(), repo_id.into()].iter().collect()
}

pub fn show() {
    let config = SETTINGS.load().backups.get_active().unwrap().clone();

    display_archives(config.clone());

    let repo_archives = if let Some(repo_archives) = REPO_ARCHIVES.load().get(&config.repo_id) {
        debug!("Archive cache is loaded from file");
        repo_archives.clone()
    } else {
        debug!("Try loading archive from file");
        let repo_archives: RepoArchives = std::fs::read_to_string(&cache_path(&config.repo_id))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        REPO_ARCHIVES.update(enclose!(
            (config, repo_archives) | ra | {
                ra.insert(config.repo_id, repo_archives);
            }
        ));
        repo_archives
    };

    if repo_archives.archives.as_ref().is_none() {
        trace!("Archives have never been retrieved");
        refresh_archives_cache(config);
    } else {
        display_archives(config);
    }
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
