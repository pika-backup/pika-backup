use adw::prelude::*;

use crate::config;

use crate::ui;
use crate::ui::backup_status;
use crate::ui::prelude::*;

use super::events;

pub fn add_list_row(list: &gtk::ListBox, file: &std::path::Path) -> gtk::Button {
    let row = adw::ActionRow::builder()
        .title(&glib::markup_escape_text(&file.to_string_lossy()))
        .activatable(false)
        .build();
    list.append(&row);

    if let Some(img) = ui::utils::file_icon(&config::absolute(file)) {
        row.add_prefix(&img);
    }

    let button = gtk::Button::builder()
        .icon_name("edit-delete-symbolic")
        .build();
    row.add_suffix(&button);
    button.set_valign(gtk::Align::Center);

    button
}

// TODO: Function has too many lines
pub fn refresh() -> Result<()> {
    let backup = BACKUP_CONFIG.load().active()?.clone();

    refresh_status();

    let include_home = backup.include.contains(&std::path::PathBuf::new());

    if include_home != main_ui().include_home().is_active() {
        main_ui().include_home().set_sensitive(false);
        main_ui().include_home().set_active(include_home);
    }

    if include_home {
        main_ui().include_home_row().remove_css_class("not-active");
    } else {
        main_ui().include_home_row().add_css_class("not-active");
    }

    // backup target ui
    if let Ok(icon) = gio::Icon::for_string(&backup.repo.icon()) {
        main_ui().detail_repo_icon().set_from_gicon(&icon);
    }

    main_ui()
        .detail_repo_row()
        .set_title(&glib::markup_escape_text(&backup.repo.title()));
    main_ui()
        .detail_repo_row()
        .set_subtitle(&glib::markup_escape_text(&backup.repo.subtitle()));

    // include list
    ui::utils::clear(&main_ui().include());
    // TODO: Warn if there a no includes, disable backup button
    for file in &backup.include {
        if *file == std::path::PathBuf::new() {
            continue;
        }

        let button = add_list_row(&main_ui().include(), file);

        let path = file.clone();
        button.connect_clicked(move |_| {
            let path = path.clone();
            Handler::run(events::on_remove_include(path))
        });
    }

    // exclude list
    ui::utils::clear(&main_ui().backup_exclude());
    for pattern in backup.exclude.clone() {
        match &pattern {
            config::Pattern::PathPrefix(file) => {
                let button = add_list_row(&main_ui().backup_exclude(), &file);
                button.connect_clicked(enclose!((file) move |_| {
                    let path = file.clone();
                    Handler::run(async move {
                        BACKUP_CONFIG.update_result(move |settings| {
                            settings
                                .active_mut()?
                                .exclude
                                .remove(&config::Pattern::PathPrefix(path.clone()));
                            Ok(())
                        })?;
                        crate::ui::write_config()?;
                        refresh()?;
                        Ok(())
                    });
                }));
            }
            config::Pattern::RegularExpression(regex) => {
                let row = adw::ActionRow::builder()
                    .title(&glib::markup_escape_text(&pattern.description()))
                    .subtitle(&gettext("Regular Expression"))
                    .activatable(false)
                    .build();

                row.add_prefix(
                    &gtk::Image::builder()
                        .icon_name("folder-saved-search")
                        .pixel_size(64)
                        .build(),
                );

                let button = gtk::Button::builder()
                    .icon_name("edit-delete-symbolic")
                    .build();
                button.set_valign(gtk::Align::Center);
                button.connect_clicked(enclose!((regex) move |_| {
                    let regex = regex.clone();
                    Handler::run(async move {
                        BACKUP_CONFIG.update_result(move |settings| {
                            settings
                                .active_mut()?
                                .exclude
                                .remove(&config::Pattern::RegularExpression(regex.clone()));
                            Ok(())
                        })?;
                        crate::ui::write_config()?;
                        refresh()?;
                        Ok(())
                    });
                }));
                row.add_suffix(&button);
                main_ui().backup_exclude().append(&row);
            }
        }
    }

    if backup.exclude.is_empty() {
        main_ui()
            .detail_exclude_stack()
            .set_visible_child(&main_ui().detail_exclude_placeholder());
    } else {
        main_ui()
            .detail_exclude_stack()
            .set_visible_child(&main_ui().backup_exclude());
    }

    Ok(())
}

pub fn refresh_status() {
    if super::is_visible() {
        if let Some(id) = ACTIVE_BACKUP_ID.load().as_ref().as_ref() {
            refresh_status_display(&backup_status::Display::new_from_id(id));
        }
    }
}

fn refresh_status_display(status: &ui::backup_status::Display) {
    main_ui()
        .detail_status_row()
        .set_title(&glib::markup_escape_text(&status.title));
    main_ui()
        .detail_status_row()
        .set_subtitle(&glib::markup_escape_text(
            &status.subtitle.clone().unwrap_or_default(),
        ));

    let running = match &status.graphic {
        ui::backup_status::Graphic::ErrorIcon(icon)
        | ui::backup_status::Graphic::WarningIcon(icon)
        | ui::backup_status::Graphic::OkIcon(icon) => {
            main_ui()
                .status_graphic()
                .set_visible_child(&main_ui().status_icon());
            main_ui().status_icon().set_from_icon_name(Some(icon));

            false
        }
        ui::backup_status::Graphic::Spinner => {
            main_ui()
                .status_graphic()
                .set_visible_child(&main_ui().status_spinner());

            true
        }
    };

    if matches!(status.graphic, ui::backup_status::Graphic::ErrorIcon(_)) {
        main_ui().status_icon().add_css_class("error-icon");
        main_ui().status_icon().remove_css_class("ok-icon");
        main_ui().status_icon().remove_css_class("warning-icon");
        main_ui().detail_hint_icon().show();
    } else if matches!(status.graphic, ui::backup_status::Graphic::WarningIcon(_)) {
        main_ui().status_icon().add_css_class("warning-icon");
        main_ui().status_icon().remove_css_class("ok-icon");
        main_ui().status_icon().remove_css_class("error-icon");
        main_ui().detail_hint_icon().show();
    } else if matches!(status.graphic, ui::backup_status::Graphic::OkIcon(_)) {
        main_ui().status_icon().add_css_class("ok-icon");
        main_ui().status_icon().remove_css_class("warning-icon");
        main_ui().status_icon().remove_css_class("error-icon");
        main_ui().detail_hint_icon().show();
    } else {
        main_ui().status_icon().remove_css_class("ok-icon");
        main_ui().status_icon().remove_css_class("error-icon");
        main_ui().status_icon().remove_css_class("warning-icon");
        main_ui().detail_hint_icon().hide();
    }

    main_ui().stop_backup_create().set_visible(running);
    main_ui().backup_run().set_sensitive(!running);
}
