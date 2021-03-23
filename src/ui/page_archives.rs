use std::collections::BTreeMap;
use std::iter::FromIterator;

use arc_swap::ArcSwap;
use gio::prelude::*;
use gtk::prelude::*;
use libhandy::prelude::*;
use once_cell::sync::Lazy;

use crate::borg;
use crate::borg::prelude::*;
use crate::config::*;
use crate::ui;
use ui::prelude::*;

static REPO_ARCHIVES: Lazy<ArcSwap<BTreeMap<borg::RepoId, RepoArchives>>> =
    Lazy::new(Default::default);

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct RepoArchives {
    archives: Option<BTreeMap<borg::ArchiveName, borg::Archive>>,
    reloading: bool,
}

impl RepoArchives {
    pub fn archives_sorted_by_date(&self) -> Option<Vec<(borg::ArchiveName, borg::Archive)>> {
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
                Handler::run(show());
            }
        });

    main_ui().refresh_archives().connect_clicked(|_| {
        Handler::run(async move {
            let config = BACKUP_CONFIG.load().get_active()?.clone();
            ui::dialog_device_missing::updated_config(
                config.clone(),
                &gettext("Update archive list"),
            )
            .await?;
            refresh_archives_cache(config.clone()).await
        });
    });

    main_ui().archives_eject_button().connect_clicked(|_| {
        Handler::run(on_eject_button_clicked());
    });

    main_ui()
        .archives_reloading_spinner()
        .connect_map(|s| s.start());
    main_ui()
        .archives_reloading_spinner()
        .connect_unmap(|s| s.stop());
}

async fn on_eject_button_clicked() -> Result<()> {
    let repo_id = BACKUP_CONFIG.load().get_active()?.repo_id.clone();

    borg::Borg::umount(&repo_id).err_to_msg(gettext("Failed to unmount repository."))?;
    ACTIVE_MOUNTS.update(|mounts| {
        mounts.remove(&repo_id);
    });
    update_eject_button()
}

pub async fn refresh_archives_cache(config: Backup) -> Result<()> {
    info!("Refreshing archives cache");

    if Some(true)
        == REPO_ARCHIVES
            .load()
            .get(&config.repo_id)
            .map(|x| x.reloading)
    {
        info!("Aborting archives cache reload because already in progress");
        return Ok(());
    } else {
        REPO_ARCHIVES.update(|repos| {
            let mut repo = repos.get(&config.repo_id).cloned().unwrap_or_default();
            repo.reloading = true;
            repos.insert(config.repo_id.clone(), repo);
        });
    }

    update_archives_spinner(config.clone());

    let result = ui::utils::borg::spawn(
        "refresh_archives_cache",
        borg::Borg::new(config.clone()),
        |borg| borg.list(100),
    )
    .await;

    archives_cache_refreshed(config.clone(), result)
}

fn update_archives_spinner(config: Backup) {
    if Ok(&config.repo_id) == BACKUP_CONFIG.load().get_active().map(|x| &x.repo_id)
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

fn archives_cache_refreshed(
    config: Backup,
    result: borg::Result<Vec<borg::Archive>>,
) -> Result<()> {
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
                    serde_json::ser::to_writer(&file, &repo_archives)
                        .err_to_msg(gettext("Failed to save cache."))?;
                }
                Err(err) => {
                    return Err(Message::new("Failed to open cache file.", err).into());
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
            return Err(Message::new("Failed to refresh archives cache.", err).into());
        }
    }

    Ok(())
}

fn show_dir(path: &std::path::Path) -> Result<()> {
    main_ui().pending_menu().hide();
    let uri = gio::File::new_for_path(&path).get_uri();

    // only open if app isn't closing in this moment
    if !**IS_SHUTDOWN.load() {
        let show_folder = || -> std::result::Result<(), _> {
            let conn = zbus::Connection::new_session()?;
            let proxy = zbus::Proxy::new(
                &conn,
                "org.freedesktop.FileManager1",
                "/org/freedesktop/FileManager1",
                "org.freedesktop.FileManager1",
            )?;
            proxy.call("ShowFolders", &(vec![uri.as_str()], ""))
        };

        show_folder().err_to_msg(gettext("Failed to open archive."))?;
    }

    Ok(())
}

async fn on_browse_archive(config: Backup, archive_name: borg::ArchiveName) -> Result<()> {
    debug!("Trying to browse an archive");

    main_ui().pending_menu().show();

    let backup_mounted = ACTIVE_MOUNTS.load().contains(&config.repo_id);

    let mut path = borg::Borg::get_mount_point(&config.repo_id);
    path.push(archive_name.as_str());

    if !backup_mounted {
        ACTIVE_MOUNTS.update(|mounts| {
            mounts.insert(config.repo_id.clone());
        });

        let mount = ui::utils::borg::spawn(
            "mount_archive",
            borg::Borg::new(config.clone()),
            move |borg| borg.mount(),
        )
        .await;

        if let Err(err) = mount {
            ACTIVE_MOUNTS.update(|mounts| {
                mounts.remove(&config.repo_id.clone());
            });
            main_ui().pending_menu().hide();

            return Err(Message::new(
                gettext("Failed to make archives available for browsing."),
                err,
            )
            .into());
        }
    }

    update_eject_button()?;

    let first_populated_dir =
        ui::utils::spawn_thread("open_archive", move || find_first_populated_dir(&path)).await?;

    show_dir(&first_populated_dir)
}

fn page_is_visible() -> bool {
    main_ui().detail_stack().get_visible_child()
        == Some(main_ui().page_archives().upcast::<gtk::Widget>())
}

fn update_eject_button() -> Result<()> {
    main_ui().archives_eject_button().set_visible(
        ACTIVE_MOUNTS
            .load()
            .contains(&BACKUP_CONFIG.load().get_active()?.repo_id),
    );
    Ok(())
}

fn display_archives(config: Backup) {
    if Ok(&config.repo_id) == BACKUP_CONFIG.load().get_active().map(|x| &x.repo_id)
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
            for (archive_name, archive) in archive_list {
                let row = libhandy::ExpanderRowBuilder::new()
                    .title(
                        &archive
                            .start
                            .to_locale()
                            .unwrap_or_else(|| archive.start.to_string()),
                    )
                    .subtitle(&format!(
                        "{hostname}, {username}",
                        hostname = archive.hostname,
                        username = archive.username
                    ))
                    .build();

                let info = |title: String, info: &str| -> libhandy::ActionRow {
                    let label = gtk::LabelBuilder::new()
                        .label(info)
                        .wrap(true)
                        .wrap_mode(pango::WrapMode::WordChar)
                        .build();
                    label.add_css_class("dim-label");

                    libhandy::ActionRowBuilder::new()
                        .title(&title)
                        .child(&label)
                        .build()
                };

                row.add(&info(gettext("Name"), archive.name.as_str()));
                row.add(&info(
                    gettext("Duration"),
                    &ui::utils::duration::plain(&(archive.end - archive.start)),
                ));
                if !archive.comment.is_empty() {
                    row.add(&info(gettext("Comment"), &archive.comment));
                }

                let browse_row = libhandy::ActionRowBuilder::new()
                    .title(&gettext("Browse saved files"))
                    .activatable(true)
                    .icon_name("folder-open-symbolic")
                    .child(&gtk::Image::from_icon_name(
                        Some("go-next-symbolic"),
                        gtk::IconSize::Button,
                    ))
                    .build();
                row.add(&browse_row);

                browse_row.connect_activated(
                    enclose!((config, archive_name) move |_| Handler::run(on_browse_archive(config.clone(), archive_name.clone()))),
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

fn cache_path(repo_id: &borg::RepoId) -> std::path::PathBuf {
    [cache_dir(), repo_id.as_str().into()].iter().collect()
}

pub async fn show() -> Result<()> {
    update_eject_button()?;

    let config = BACKUP_CONFIG.load().get_active()?.clone();

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
        refresh_archives_cache(config).await?;
    } else {
        display_archives(config);
    }

    Ok(())
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
