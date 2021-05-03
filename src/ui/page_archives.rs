use gio::prelude::*;
use gtk::prelude::*;
use libhandy::prelude::*;

use crate::borg;
use crate::borg::prelude::*;
use crate::config::*;
use crate::ui;
use ui::prelude::*;
use ui::utils::repo_cache::RepoCache;

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
            refresh_archives_cache(config).await
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

    if Some(true) == REPO_CACHE.load().get(&config.repo_id).map(|x| x.reloading) {
        info!("Aborting archives cache reload because already in progress");
        return Ok(());
    } else {
        REPO_CACHE.update(|repos| {
            repos
                .entry(config.repo_id.clone())
                .or_insert_with_key(RepoCache::new)
                .reloading = true;
        });
    }
    ui_update_archives_spinner();

    let result =
        ui::utils::borg::exec("Update archive list", config.clone(), |borg| borg.list(100))
            .await
            .into_message(gettext("Failed to refresh archives cache."));

    REPO_CACHE.update(|repos| {
        repos
            .entry(config.repo_id.clone())
            .or_insert_with_key(RepoCache::new)
            .reloading = false;
    });

    ui_update_archives_spinner();

    let archives = result?;

    REPO_CACHE.update(enclose!((config) move |repos| {
        let mut repo_archives = repos
            .entry(config.repo_id.clone())
            .or_insert_with_key(RepoCache::new);

        repo_archives.archives = Some(
            archives
                .iter()
                .map(|x| (x.name.clone(), x.clone()))
                .collect(),
        );

    }));
    info!("Archives cache refreshed");

    RepoCache::write(&config.repo_id)?;

    ui_display_archives(&config.repo_id);

    Ok(())
}

fn ui_update_archives_spinner() {
    if page_is_visible() {
        if let Ok(repo_id) = BACKUP_CONFIG.load().get_active().map(|x| &x.repo_id) {
            let reloading = REPO_CACHE
                .load()
                .get(repo_id)
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

async fn on_browse_archive(archive_name: borg::ArchiveName) -> Result<()> {
    let configs = BACKUP_CONFIG.load();
    let config = configs.get_active()?;
    let repo_id = &config.repo_id;

    debug!("Trying to browse an archive");

    let backup_mounted = ACTIVE_MOUNTS.load().contains(repo_id);

    let mut path = borg::Borg::get_mount_point(repo_id);
    path.push(archive_name.as_str());

    if !backup_mounted {
        ACTIVE_MOUNTS.update(|mounts| {
            mounts.insert(repo_id.clone());
        });

        main_ui().pending_menu().show();
        let mount = ui::utils::borg::exec(gettext("Browse archive"), config.clone(), move |borg| {
            borg.mount()
        })
        .await;

        if mount.is_err() {
            ACTIVE_MOUNTS.update(|mounts| {
                mounts.remove(repo_id);
            });
            main_ui().pending_menu().hide();
        }

        mount.into_message(gettext("Failed to make archives available for browsing."))?;
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

fn ui_display_archives(repo_id: &borg::RepoId) {
    if Ok(repo_id) != BACKUP_CONFIG.load().get_active().map(|x| &x.repo_id) || !page_is_visible() {
        debug!("Not displaying archive list because it's not visible");
        return;
    }

    debug!("Displaying archive list from cache");
    let repo_cache = RepoCache::get(repo_id);

    ui::utils::clear(&main_ui().archive_list());
    ui_update_archives_spinner();

    for (archive_name, archive) in repo_cache.archives_sorted_by_date() {
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
                    enclose!((repo_id, archive_name) move |_| Handler::run(on_browse_archive(archive_name.clone()))),
                );

        main_ui().archive_list().add(&row);
    }

    main_ui().archive_list().show_all();

    if !repo_cache.archives_sorted_by_date().is_empty() {
        main_ui()
            .archives_stack()
            .set_visible_child(&main_ui().archive_list());
    } else {
        main_ui()
            .archives_stack()
            .set_visible_child(&main_ui().archive_list_placeholder());
    }
}

pub async fn show() -> Result<()> {
    update_eject_button()?;

    ui::utils::clear(&main_ui().archive_list());

    let config = BACKUP_CONFIG.load().get_active()?.clone();
    let repo_archives = RepoCache::get(&config.repo_id);

    let result = if repo_archives.archives.as_ref().is_none() {
        trace!("Archives have never been retrieved");
        refresh_archives_cache(config.clone()).await
    } else {
        Ok(())
    };

    ui_display_archives(&config.repo_id);

    result
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
