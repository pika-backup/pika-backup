use gtk::prelude::*;
use libhandy::prelude::*;

use crate::shared;
use crate::ui;
use crate::ui::detail;
use crate::ui::globals::*;
use crate::ui::new_backup;
use crate::ui::prelude::*;

pub fn init() {
    refresh();

    main_ui().main_backups().connect_row_activated(|_, row| {
        let name = row.get_widget_name();
        detail::view_backup_conf(&name)
    });

    main_ui()
        .content_leaflet()
        .connect_property_folded_notify(on_leaflet_folded);

    main_ui()
        .add_backup_left()
        .connect_clicked(|_| new_backup::new_backup());
    main_ui()
        .add_backup_right()
        .connect_clicked(|_| new_backup::new_backup());
    main_ui()
        .add_backup_empty()
        .connect_clicked(|_| new_backup::new_backup());

    main_ui().remove_backup().connect_clicked(on_remove_backup);
}

pub fn refresh() {
    if SETTINGS.load().backups.len() == 1 {
        ACTIVE_BACKUP_ID.update(|active_id| {
            *active_id = SETTINGS
                .load()
                .backups
                .values()
                .next()
                .map(|x| x.id.clone())
        });
    }

    if SETTINGS.load().backups.is_empty() {
        main_ui()
            .content_stack()
            .set_visible_child(&main_ui().overview_empty());
    } else if ACTIVE_BACKUP_ID.load().is_none() {
        main_ui()
            .content_stack()
            .set_visible_child(&main_ui().page_start());
    } else {
        main_ui()
            .content_stack()
            .set_visible_child(&main_ui().page_main());
    }

    if SETTINGS.load().backups.len() > 1 {
        main_ui().leaflet_left().show();
    } else {
        main_ui().leaflet_left().hide();
    }

    refresh_list();
}

fn on_leaflet_folded(leaflet: &libhandy::Leaflet) {
    main_ui()
        .main_backups()
        .set_selection_mode(if leaflet.get_folded() {
            gtk::SelectionMode::None
        } else {
            gtk::SelectionMode::Single
        });
}

fn on_remove_backup(_button: &gtk::Button) {
    let delete = ui::utils::dialog_yes_no(gettext(
        "Are you sure you want to delete this backup configuration?",
    ));

    if delete {
        if let Some(config) = SETTINGS.get().backups.get_active() {
            SETTINGS.update(move |s| {
                s.backups.remove(&config.id);
            });

            ui::utils::dialog_catch_err(
                ui::utils::secret_service_delete_passwords(config),
                "Failed to remove potentially remaining passwords from key storage.",
            );

            ACTIVE_BACKUP_ID.update(|active_id| *active_id = None);
            ui::write_config();
            ui::config_list::refresh();
            ui::headerbar::update();
        } else {
            ui::utils::dialog_error(gettext("No active backup to delete."));
        }
    }
}

fn refresh_list() {
    let list = main_ui().main_backups();
    ui::utils::clear(&list);

    if SETTINGS.load().backups.is_empty() {
        main_ui().overview_empty().show();
    } else {
        main_ui().overview_empty().hide();
    }

    for backup in SETTINGS.load().backups.values() {
        let (_, horizontal_box) = ui::utils::add_list_box_row(&list, Some(&backup.id), 0);

        if let Some(path) = backup.include_dirs().get(0) {
            if let Some(icon) = ui::utils::file_icon(&shared::absolute(path), gtk::IconSize::Dialog)
            {
                horizontal_box.add(&icon);
            }
        }

        let includes = backup
            .include
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .map(|p| if p.is_empty() { gettext("Home") } else { p })
            .collect::<Vec<String>>()
            .join(", ");

        let mut location = "".to_string();

        if let shared::BackupRepo::Local {
            ref label,
            ref device,
            ..
        } = backup.repo
        {
            if let Some(ref label) = label {
                location.push_str(label);
            }

            if let Some(ref device) = device {
                if !location.is_empty() {
                    location.push_str(&gettext(" on "));
                }
                location.push_str(device);
            }
        }

        if location.is_empty() {
            location.push_str(&backup.repo.to_string());
        }

        let (vertical_box, _, _) = ui::utils::list_vertical_box(Some(&includes), Some(&location));
        horizontal_box.add(&vertical_box);
    }
    list.show_all();
}
