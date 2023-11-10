use crate::ui::backup_status;
use crate::ui::prelude::*;
use adw::prelude::*;

use super::cache;
use super::events;
use crate::ui::utils::repo_cache::RepoCache;
use crate::{borg, config, ui};

pub async fn show() -> Result<()> {
    update_eject_button().await?;

    ui::utils::clear(&main_ui().archive_list());

    let config = BACKUP_CONFIG.load().active()?.clone();

    // location info

    update_info(&config);

    // archives list

    let repo_archives = RepoCache::get(&config.repo_id);

    let result = if repo_archives.archives.as_ref().is_none() {
        trace!("Archives have never been retrieved");
        cache::refresh_archives(config.clone(), None).await
    } else {
        Ok(())
    };

    ui_display_archives(&config.repo_id);
    refresh_status();

    result
}

pub fn refresh_status() {
    if super::is_visible() {
        if let Some(id) = ACTIVE_BACKUP_ID.load().as_ref().as_ref() {
            main_ui()
                .check_status()
                .set_from_backup_status(&backup_status::Display::new_check_status_from_id(id));
            main_ui().dialog_check_result().reload();

            BORG_OPERATION.with(|ops| {
                let op = ops.load().get(id).cloned();

                let running =
                    matches!(op, Some(ref op) if op.task_kind() == borg::task::Kind::Check);

                main_ui().archives_check_now().set_visible(!running);
                main_ui().archives_check_now().set_sensitive(op.is_none());
                main_ui().archives_check_abort().set_visible(running);
            });
        }

        if let Ok(config) = BACKUP_CONFIG.load().active() {
            let is_mounted = ACTIVE_MOUNTS.load().contains(&config.repo_id);
            main_ui().archives_eject_button().set_visible(is_mounted);
        }
    }
}

pub fn update_info(config: &config::Backup) {
    if let Ok(icon) = gio::Icon::for_string(&config.repo.icon()) {
        main_ui().archives_location_icon().set_from_gicon(&icon);
    }

    main_ui()
        .archives_location_suffix_title()
        .set_visible(false);
    main_ui()
        .archives_location_suffix_subtitle()
        .set_visible(false);
    main_ui().archives_fs_usage().set_visible(false);
    Handler::run(update_df());

    main_ui()
        .archives_location_title()
        .set_label(&config.title());
    main_ui()
        .archives_location_subtitle()
        .set_label(&config.repo.subtitle());

    match config.archive_prefix.to_string() {
        prefix if !prefix.is_empty() => {
            main_ui().archives_prefix().set_label(&prefix);
        }
        _ => {
            main_ui().archives_prefix().set_label(&gettext("None"));
        }
    }
}

pub async fn show_dir(path: &std::path::Path) -> Result<()> {
    main_ui().pending_menu().set_visible(false);
    let file = gio::File::for_path(path);

    // Only open if app isn't closing in this moment
    if !**IS_SHUTDOWN.load() {
        gtk::FileLauncher::new(Some(&file))
            .launch_future(Some(&main_ui().window()))
            .await
            .err_to_msg(gettext("Failed to open archive."))?;
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

pub async fn update_eject_button() -> Result<()> {
    ui::utils::borg::cleanup_mounts().await?;
    refresh_status();
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

        if archive.name.as_str().ends_with(".checkpoint") {
            let checkpoint_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            checkpoint_box.add_css_class("tag-box");
            checkpoint_box.set_valign(gtk::Align::Center);

            let info_tag = gtk::Label::new(Some(&gettext("Incomplete Archive")));
            info_tag.add_css_class("tag");

            checkpoint_box.append(&info_tag);
            row.add_suffix(&checkpoint_box);
        }

        let info = |title: String, info: &str| -> adw::ActionRow {
            let label = gtk::Label::builder()
                .label(info)
                .wrap(true)
                .wrap_mode(gtk::pango::WrapMode::WordChar)
                .natural_wrap_mode(gtk::NaturalWrapMode::None)
                .build();
            label.add_css_class("dim-label");

            let row = adw::ActionRow::builder().title(title).build();
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
            .build();

        browse_row.add_prefix(&gtk::Image::from_icon_name("folder-open-symbolic"));
        browse_row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));

        row.add_row(&browse_row);

        browse_row.connect_activated(
            enclose!((archive_name) move |_| Handler::run(events::browse_archive(archive_name.clone()))),
        );

        let delete_row = adw::ActionRow::builder()
            .title(&gettext("Delete archive"))
            .activatable(true)
            .build();

        delete_row.add_prefix(&gtk::Image::from_icon_name("edit-delete-symbolic"));
        delete_row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));

        row.add_row(&delete_row);

        delete_row.connect_activated(
            enclose!((archive_name) move |_| Handler::run(events::delete_archive(archive_name.clone(), archive.clone()))),
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

        main_ui().archives_location_suffix_title().set_visible(true);
        main_ui()
            .archives_location_suffix_subtitle()
            .set_visible(true);
        main_ui().archives_fs_usage().set_visible(true);
    }

    Ok(())
}
