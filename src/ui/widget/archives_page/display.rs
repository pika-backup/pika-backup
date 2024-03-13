use crate::ui::backup_status;
use crate::ui::prelude::*;
use crate::ui::App;

use adw::prelude::*;
use adw::subclass::prelude::*;

use super::imp;
use crate::ui::utils::repo_cache::RepoCache;
use crate::{borg, config, ui};

impl imp::ArchivesPage {
    pub async fn show(&self) -> Result<()> {
        ui::utils::clear(&self.list);

        let config = BACKUP_CONFIG.load().active()?.clone();

        // location info

        self.update_info(&config);

        // Eject button

        self.update_eject_button().await?;

        // archives list

        let repo_archives = RepoCache::get(&config.repo_id);

        let result = if repo_archives.archives.as_ref().is_none() {
            trace!("Archives have never been retrieved");
            self.refresh_archives(config.clone(), None).await
        } else {
            Ok(())
        };

        self.ui_display_archives(&config.repo_id);
        self.refresh_status();

        result
    }

    pub fn refresh_status(&self) {
        if self.obj().is_visible() {
            if let Some(id) = ACTIVE_BACKUP_ID.load().as_ref().as_ref() {
                self.check_status_row
                    .set_from_backup_status(&backup_status::Display::new_check_status_from_id(id));
                self.check_result_dialog.reload();

                BORG_OPERATION.with(|ops| {
                    let op = ops.load().get(id).cloned();

                    let running =
                        matches!(op, Some(ref op) if op.task_kind() == borg::task::Kind::Check);

                    self.check_button.set_visible(!running);
                    self.check_button.set_sensitive(op.is_none());
                    self.check_abort_button.set_visible(running);
                });
            }

            if let Ok(config) = BACKUP_CONFIG.load().active() {
                let is_mounted = ACTIVE_MOUNTS.load().contains(&config.repo_id);
                self.eject_button.set_visible(is_mounted);
            }
        }
    }

    pub fn update_info(&self, config: &config::Backup) {
        if let Ok(icon) = gio::Icon::for_string(&config.repo.icon()) {
            self.location_icon.set_from_gicon(&icon);
        }

        self.location_suffix_title.set_visible(false);
        self.location_suffix_subtitle.set_visible(false);
        self.fs_usage.set_visible(false);
        let obj = self.obj().clone();
        Handler::run(async move { obj.imp().update_df().await });

        self.location_title.set_label(&config.title());
        self.location_subtitle.set_label(&config.repo.subtitle());

        match config.archive_prefix.to_string() {
            prefix if !prefix.is_empty() => {
                self.prefix_label.set_label(&prefix);
            }
            _ => {
                self.prefix_label.set_label(&gettext("None"));
            }
        }
    }

    pub async fn show_dir(&self, path: &std::path::Path) -> Result<()> {
        main_ui().page_detail().show_pending_menu(false);
        let file = gio::File::for_path(path);
        let app = App::default();

        // Only open if app isn't closing in this moment
        if !app.in_shutdown() {
            gtk::FileLauncher::new(Some(&file))
                .launch_future(Some(&main_ui().window()))
                .await
                .err_to_msg(gettext("Failed to open archive."))?;
        }

        Ok(())
    }

    pub fn ui_update_archives_spinner(&self) {
        if self.obj().is_visible() {
            if let Ok(repo_id) = BACKUP_CONFIG.load().active().map(|x| &x.repo_id) {
                let reloading = REPO_CACHE
                    .load()
                    .get(repo_id)
                    .map(|x| x.reloading)
                    .unwrap_or_default();

                if reloading {
                    self.reloading_stack
                        .set_visible_child(&*self.reloading_spinner);
                } else {
                    self.reloading_stack
                        .set_visible_child(&*self.refresh_archives_button);
                }
            }
        }
    }

    pub async fn update_eject_button(&self) -> Result<()> {
        ui::utils::borg::cleanup_mounts().await?;
        self.refresh_status();
        Ok(())
    }

    pub fn ui_display_archives(&self, repo_id: &borg::RepoId) {
        if Ok(repo_id) != BACKUP_CONFIG.load().active().map(|x| &x.repo_id)
            || !self.obj().is_visible()
        {
            debug!("Not displaying archive list because it's not visible");
            return;
        }

        debug!("Displaying archive list from cache");
        let repo_cache = RepoCache::get(repo_id);

        ui::utils::clear(&self.list);
        self.ui_update_archives_spinner();

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

            let obj = self.obj();
            browse_row.connect_activated(
                glib::clone!(@weak obj, @strong archive_name => move |_| {
                    let name = archive_name.clone();
                    Handler::run(async move { obj.imp().browse_archive(name).await});
                }),
            );

            let delete_row = adw::ActionRow::builder()
                .title(&gettext("Delete archive"))
                .activatable(true)
                .build();

            delete_row.add_prefix(&gtk::Image::from_icon_name("edit-delete-symbolic"));
            delete_row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));

            row.add_row(&delete_row);

            delete_row.connect_activated(glib::clone!(@weak obj => move |_| {
                let name = archive_name.clone();
                let archive = archive.clone();
                Handler::run(async move { obj.imp().delete_archive(name, archive).await });
            }));

            self.list.append(&row);
        }

        if !repo_cache.archives_sorted_by_date().is_empty() {
            self.list_stack.set_visible_child(&*self.list);
        } else {
            self.list_stack.set_visible_child(&*self.list_placeholder);
        }
    }

    pub async fn update_df(&self) -> Result<()> {
        let backups = BACKUP_CONFIG.load();
        let config = backups.active()?;

        if let Some(df) = ui::utils::df::cached_or_lookup(config).await {
            self.location_suffix_title
                .set_label(&gettextf("{} Available", &[&glib::format_size(df.avail)]));
            self.location_suffix_subtitle
                .set_label(&gettextf("{} Total", &[&glib::format_size(df.size)]));

            self.fs_usage
                .set_value(1.0 - df.avail as f64 / df.size as f64);

            self.location_suffix_title.set_visible(true);
            self.location_suffix_subtitle.set_visible(true);
            self.fs_usage.set_visible(true);
        }

        Ok(())
    }
}
