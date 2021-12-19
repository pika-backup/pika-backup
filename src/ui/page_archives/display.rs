use crate::ui::prelude::*;
use adw::prelude::*;

use super::cache;
use super::events;
use crate::borg;
use crate::ui;
use crate::ui::utils::repo_cache::RepoCache;

pub async fn show() -> Result<()> {
    update_eject_button()?;

    ui::utils::clear(&main_ui().archive_list());

    let config = BACKUP_CONFIG.load().active()?.clone();

    // location info

    if let Ok(icon) = gio::Icon::for_string(&config.repo.icon()) {
        main_ui().archives_location_icon().set_from_gicon(&icon);
    }

    Handler::run(update_df());

    main_ui()
        .archives_location_title()
        .set_label(&config.repo.title());
    main_ui()
        .archives_location_subtitle()
        .set_label(&config.repo.subtitle());

    // archives list

    let repo_archives = RepoCache::get(&config.repo_id);

    let result = if repo_archives.archives.as_ref().is_none() {
        trace!("Archives have never been retrieved");
        cache::refresh_archives(config.clone()).await
    } else {
        Ok(())
    };

    ui_display_archives(&config.repo_id);

    result
}

pub fn show_dir(path: &std::path::Path) -> Result<()> {
    main_ui().pending_menu().hide();
    let uri = gio::File::for_path(&path).uri();

    // only open if app isn't closing in this moment
    if !**IS_SHUTDOWN.load() {
        let show_folder = || -> std::result::Result<(), _> {
            let conn = zbus::blocking::Connection::session()?;
            let proxy = zbus::blocking::Proxy::new(
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

pub fn ui_update_archives_spinner() {
    if super::is_visible() {
        if let Ok(repo_id) = BACKUP_CONFIG.load().active().map(|x| &x.repo_id) {
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

pub fn update_eject_button() -> Result<()> {
    main_ui().archives_eject_button().set_visible(
        ACTIVE_MOUNTS
            .load()
            .contains(&BACKUP_CONFIG.load().active()?.repo_id),
    );
    Ok(())
}

pub fn ui_display_archives(repo_id: &borg::RepoId) {
    if Ok(repo_id) != BACKUP_CONFIG.load().active().map(|x| &x.repo_id) || !super::is_visible() {
        debug!("Not displaying archive list because it's not visible");
        return;
    }

    debug!("Displaying archive list from cache");
    let repo_cache = RepoCache::get(repo_id);

    ui::utils::clear(&main_ui().archive_list());
    ui_update_archives_spinner();

    for (archive_name, archive) in repo_cache.archives_sorted_by_date() {
        let row = adw::ExpanderRow::builder()
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

        let info = |title: String, info: &str| -> adw::ActionRow {
            let label = gtk::Label::builder()
                .label(info)
                .wrap(true)
                .wrap_mode(pango::WrapMode::WordChar)
                .build();
            label.add_css_class("dim-label");

            let row = adw::ActionRow::builder().title(&title).build();
            row.add_suffix(&label);
            row
        };

        row.add_row(&info(gettext("Name"), archive.name.as_str()));
        row.add_row(&info(
            gettext("Duration"),
            &ui::utils::duration::plain(&(archive.end - archive.start)),
        ));
        if !archive.comment.is_empty() {
            row.add_row(&info(gettext("Comment"), &archive.comment));
        }

        let browse_row = adw::ActionRow::builder()
            .title(&gettext("Browse saved files"))
            .activatable(true)
            .icon_name("folder-open-symbolic")
            .build();
        browse_row.add_suffix(&gtk::Image::from_icon_name(Some("go-next-symbolic")));

        row.add_row(&browse_row);

        browse_row.connect_activated(
            enclose!((archive_name) move |_| Handler::run(events::browse_archive(archive_name.clone()))),
        );

        main_ui().archive_list().append(&row);
    }

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

pub async fn update_df() -> Result<()> {
    let backups = BACKUP_CONFIG.load();
    let config = backups.active()?;

    if let Some(df) = ui::utils::df::cached_or_lookup(config).await {
        main_ui()
            .archives_location_suffix_title()
            .set_label(&gettextf("{} Available", &[&glib::format_size(df.avail)]));
        main_ui()
            .archives_location_suffix_subtitle()
            .set_label(&gettextf("{} Total", &[&glib::format_size(df.size)]));

        main_ui()
            .archives_fs_usage()
            .set_value(1.0 - df.avail as f64 / df.size as f64);
    }

    Ok(())
}
