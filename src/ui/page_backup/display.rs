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
        .title(&title)
        .subtitle(&subtitle)
        .activatable(false)
        .build();
    list.append(&row);

    if let Some(image) = crate::utils::file_symbolic_icon(&config::path_absolute(file))
        .map(|x| gtk::Image::from_gicon(&x))
    {
        image.add_css_class("row-icon");
        row.add_prefix(&image);
    }

    let button = gtk::Button::builder()
        .icon_name("edit-delete-symbolic")
        .valign(gtk::Align::Center)
        .build();
    button.add_css_class("flat");
    row.add_suffix(&button);

    button
}

// TODO: Function has too many lines
pub fn refresh() -> Result<()> {
    let backup = BACKUP_CONFIG.load().active()?.clone();

    refresh_status();

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
    for pattern in backup.exclude.clone() {
        let row = adw::ActionRow::builder()
            .title(&glib::markup_escape_text(&pattern.description()))
            .subtitle(&pattern.kind())
            .activatable(false)
            .build();

        if let Some(image) = pattern.symbolic_icon().map(|x| gtk::Image::from_gicon(&x)) {
            image.add_css_class("row-icon");
            row.add_prefix(&image);
        }

        let button = gtk::Button::builder()
            .icon_name("edit-delete-symbolic")
            .valign(gtk::Align::Center)
            .build();
        button.add_css_class("flat");
        let pattern_ = pattern.clone();
        button.connect_clicked(move |_| {
            let pattern = pattern_.clone();
            Handler::run(async move {
                BACKUP_CONFIG.update_result(move |settings| {
                    settings.active_mut()?.exclude.remove(&pattern.clone());
                    Ok(())
                })?;
                crate::ui::write_config()?;
                refresh()?;
                Ok(())
            });
        });
        row.add_suffix(&button);
        main_ui().backup_exclude().append(&row);
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
