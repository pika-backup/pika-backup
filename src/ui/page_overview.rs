use gtk::prelude::*;

use crate::shared;
use crate::ui;
use crate::ui::globals::*;
use crate::ui::prelude::*;

pub fn init() {
    if SETTINGS.load().backups.len() > 1 {
        main_ui()
            .main_stack()
            .set_visible_child(&main_ui().page_overview());
        refresh();
    } else if let Some(ref config) = SETTINGS.load().backups.values().next() {
        ui::page_detail::view_backup_conf(&config.id);
    } else {
        main_ui()
            .main_stack()
            .set_visible_child(&main_ui().page_overview_empty());
    }

    main_ui()
        .main_stack()
        .connect_property_visible_child_notify(on_main_stack_changed);

    main_ui()
        .main_backups()
        .connect_row_activated(|_, row| ui::page_detail::view_backup_conf(&row.get_widget_name()));

    main_ui()
        .add_backup()
        .connect_clicked(|_| ui::dialog_add_config::new_backup());
    main_ui()
        .add_backup_empty()
        .connect_clicked(|_| ui::dialog_add_config::new_backup());

    main_ui().remove_backup().connect_clicked(on_remove_backup);

    main_ui()
        .main_stack()
        .connect_property_transition_running_notify(on_transition);
}

fn on_transition(stack: &gtk::Stack) {
    if (!stack.get_transition_running())
        && stack.get_visible_child() != Some(main_ui().page_overview().upcast::<gtk::Widget>())
    {
        // get rid of potential GtkSpinner's for performance reasons
        ui::utils::clear(&main_ui().main_backups());
    }
}

fn on_main_stack_changed(stack: &gtk::Stack) {
    if stack.get_visible_child() == Some(main_ui().page_overview().upcast::<gtk::Widget>()) {
        refresh();
    }
}

fn on_remove_backup(_button: &gtk::ModelButton) {
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

            if SETTINGS.load().backups.is_empty() {
                main_ui()
                    .main_stack()
                    .set_visible_child(&main_ui().page_overview_empty());
            } else {
                main_ui()
                    .main_stack()
                    .set_visible_child(&main_ui().page_overview());
            };
        } else {
            ui::utils::dialog_error(gettext("No active backup to delete."));
        }
    }
}

fn refresh() {
    let list = main_ui().main_backups();
    ui::utils::clear(&list);

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
