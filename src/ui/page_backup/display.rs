use adw::prelude::*;

use crate::config;

use crate::ui;
use crate::ui::backup_status;
use crate::ui::prelude::*;

use super::events;

pub fn add_list_row(list: &gtk::ListBox, file: &std::path::Path) -> gtk::Button {
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
pub fn refresh() -> Result<()> {
    let backup = BACKUP_CONFIG.load().active()?.clone();

    refresh_status();
    refresh_disk_status();

    // backup target ui
    if let Ok(icon) = gio::Icon::for_string(&backup.repo.icon()) {
        main_ui().detail_repo_icon().set_from_gicon(&icon);
    }

    main_ui()
        .detail_repo_row()
        .set_title(&glib::markup_escape_text(&backup.title()));
    main_ui()
        .detail_repo_row()
        .set_subtitle(&glib::markup_escape_text(&backup.repo.subtitle()));

    // include list
    ui::utils::clear(&main_ui().include());

    for file in &backup.include {
        let button = add_list_row(&main_ui().include(), file);

        let path = file.clone();
        button.connect_clicked(move |_| {
            let path = path.clone();
            Handler::run(events::on_remove_include(path))
        });
    }

    // exclude list
    ui::utils::clear(&main_ui().backup_exclude());
    for exclude in backup.exclude {
        let row = adw::ActionRow::builder()
            .title(glib::markup_escape_text(&exclude.description()))
            .subtitle(&exclude.kind())
            .activatable(false)
            .build();

        if let Some(image) = exclude.symbolic_icon() {
            image.add_css_class("row-icon");
            row.add_prefix(&image);
        }

        if let config::Exclude::Pattern(ref pattern) = exclude {
            match pattern {
                config::Pattern::Fnmatch(_) | config::Pattern::RegularExpression(_) => {
                    // Make Regex and Shell patterns editable
                    let edit_button = gtk::Button::builder()
                        .icon_name("document-edit-symbolic")
                        .valign(gtk::Align::Center)
                        .tooltip_text(gettext("Edit Pattern"))
                        .build();

                    edit_button.add_css_class("flat");

                    // Edit patterns
                    edit_button.connect_clicked(clone!(@strong exclude => move |_| {
                        ui::dialog_exclude_pattern::show(Some(exclude.clone()));
                    }));

                    row.add_suffix(&edit_button);
                }
                _ => {}
            }
        }

        let delete_button = gtk::Button::builder()
            .icon_name("edit-delete-symbolic")
            .valign(gtk::Align::Center)
            .tooltip_text(gettext("Remove From List"))
            .build();

        delete_button.add_css_class("flat");

        let exclude_ = exclude.clone();
        delete_button.connect_clicked(move |_| {
            let pattern = exclude_.clone();
            Handler::run(async move {
                BACKUP_CONFIG.try_update(move |settings| {
                    settings.active_mut()?.exclude.remove(&pattern.clone());
                    Ok(())
                })?;
                crate::ui::write_config()?;
                refresh()?;
                Ok(())
            });
        });
        row.add_suffix(&delete_button);

        main_ui().backup_exclude().append(&row);
    }

    Ok(())
}

pub fn refresh_disk_status() {
    if let Ok(backup) = BACKUP_CONFIG.load().active().cloned() {
        let operation_running =
            BORG_OPERATION.with(|operations| operations.load().get(&backup.id).is_some());

        main_ui()
            .backup_disk_eject_button()
            .set_visible(!operation_running && backup.repo.is_drive_ejectable().unwrap_or(false));

        main_ui()
            .backup_disk_disconnected()
            .set_visible(!backup.repo.is_drive_connected().unwrap_or(true));
    }
}

pub fn refresh_status() {
    if super::is_visible() {
        if let Some(id) = ACTIVE_BACKUP_ID.load().as_ref().as_ref() {
            refresh_status_display(&backup_status::Display::new_from_id(id));
        }
    }
}

fn refresh_status_display(status: &ui::backup_status::Display) {
    main_ui().detail_status_row().set_from_backup_status(status);

    let running = matches!(&status.graphic, ui::backup_status::Graphic::Spinner);
    main_ui().stop_backup_create().set_visible(running);
    main_ui().backup_run().set_sensitive(!running);
    main_ui().detail_hint_icon().set_visible(!running);
}
