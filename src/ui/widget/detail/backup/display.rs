use adw::prelude::*;
use adw::subclass::prelude::*;

use crate::config;
use crate::ui;
use crate::ui::backup_status;
use crate::ui::prelude::*;
use crate::ui::widget::ExcludeDialog;

use super::imp;

impl imp::BackupPage {
    pub fn add_list_row(&self, list: &gtk::ListBox, file: &std::path::Path) -> gtk::Button {
        let title = if file == std::path::Path::new("") {
            gettext("Home")
        } else {
            file.display().to_string()
        };

        let subtitle = if file == std::path::Path::new("") {
            gettext("Usually contains all personal data")
        } else {
            String::new()
        };

        let row = adw::ActionRow::builder()
            .use_markup(false)
            .activatable(false)
            .build();

        row.set_title(&title);
        row.set_subtitle(&subtitle);
        list.append(&row);

        if let Some(image) = crate::utils::file_symbolic_icon(&config::absolute(file)) {
            image.add_css_class("row-icon");
            row.add_prefix(&image);
        }

        let button = gtk::Button::builder()
            .icon_name("edit-delete-symbolic")
            .valign(gtk::Align::Center)
            .tooltip_text(gettext("Remove Directory"))
            .build();
        button.add_css_class("flat");
        row.add_suffix(&button);

        button
    }

    // TODO: Function has too many lines
    pub fn refresh(&self) -> Result<()> {
        let obj = self.obj();
        let backup = BACKUP_CONFIG.load().active()?.clone();

        self.refresh_status();
        self.refresh_disk_status();

        // backup target ui
        if let Ok(icon) = gio::Icon::for_string(&backup.repo.icon()) {
            self.detail_repo_icon.set_from_gicon(&icon);
        }

        self.detail_repo_row
            .set_title(&glib::markup_escape_text(&backup.title()));
        self.detail_repo_row
            .set_subtitle(&glib::markup_escape_text(&backup.repo.subtitle()));

        // include list
        ui::utils::clear(&self.include_list);

        for file in &backup.include {
            let button = self.add_list_row(&self.include_list, file);

            let path = file.clone();
            button.connect_clicked(glib::clone!(
                #[weak]
                obj,
                move |_| {
                    let path = path.clone();
                    Handler::run(async move { obj.imp().on_remove_include(path).await })
                }
            ));
        }

        // exclude list
        ui::utils::clear(&self.exclude_list);
        let window = self.obj().app_window();

        for exclude in backup.exclude {
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(&exclude.description()))
                .subtitle(exclude.kind())
                .activatable(false)
                .build();

            if let Some(image) = exclude.symbolic_icon() {
                image.add_css_class("row-icon");
                row.add_prefix(&image);
            }

            if let config::Exclude::Pattern(ref pattern) = exclude
                && matches!(
                    pattern,
                    config::Pattern::Fnmatch(_) | config::Pattern::RegularExpression(_)
                )
            {
                // Make Regex and Shell patterns editable
                let edit_button = gtk::Button::builder()
                    .icon_name("document-edit-symbolic")
                    .valign(gtk::Align::Center)
                    .tooltip_text(gettext("Edit Pattern"))
                    .build();

                edit_button.add_css_class("flat");

                // Edit patterns
                edit_button.connect_clicked(clone!(
                    #[strong]
                    exclude,
                    #[weak]
                    window,
                    move |_| {
                        let config = BACKUP_CONFIG.load_full();
                        let Ok(active) = config.active() else {
                            return;
                        };

                        let dialog = ExcludeDialog::new(active);
                        dialog.present_edit_exclude(&window, exclude.clone());
                    }
                ));

                row.add_suffix(&edit_button);
            }

            let delete_button = gtk::Button::builder()
                .icon_name("edit-delete-symbolic")
                .valign(gtk::Align::Center)
                .tooltip_text(gettext("Remove From List"))
                .build();

            delete_button.add_css_class("flat");

            let exclude_ = exclude.clone();
            let obj = self.obj();
            delete_button.connect_clicked(glib::clone!(
                #[weak]
                obj,
                move |_| {
                    let pattern = exclude_.clone();
                    Handler::run(async move {
                        BACKUP_CONFIG
                            .try_update(move |settings| {
                                settings.active_mut()?.exclude.remove(&pattern.clone());
                                Ok(())
                            })
                            .await?;
                        obj.imp().refresh()?;
                        Ok(())
                    });
                }
            ));
            row.add_suffix(&delete_button);

            self.exclude_list.append(&row);
        }

        Ok(())
    }

    pub fn refresh_disk_status(&self) {
        if let Ok(backup) = BACKUP_CONFIG.load().active().cloned() {
            let operation_running =
                BORG_OPERATION.with(|operations| operations.load().get(&backup.id).is_some());

            self.backup_disk_eject_button.set_visible(
                !operation_running && backup.repo.is_drive_ejectable().unwrap_or(false),
            );

            self.backup_disk_disconnected
                .set_visible(!backup.repo.is_drive_connected().unwrap_or(true));
        }
    }

    pub fn refresh_status(&self) {
        if self.obj().is_visible()
            && let Some(id) = ACTIVE_BACKUP_ID.load().as_ref().as_ref()
        {
            let display = backup_status::Display::new_from_id(id);
            self.refresh_status_display(&display);

            if self.detail_dialog.is_mapped() {
                self.detail_dialog.refresh_status_display(&display);
            }

            self.backup_status.replace(Some(display));
        }
    }

    fn refresh_status_display(&self, status: &ui::backup_status::Display) {
        self.detail_status_row.set_from_backup_status(status);

        let running = matches!(&status.graphic, ui::backup_status::Graphic::Spinner);
        self.abort_button.set_visible(running);
        self.backup_button.set_visible(!running);
        self.detail_hint_icon.set_visible(!running);
    }
}
