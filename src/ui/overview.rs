use gtk::prelude::*;

use crate::shared;
use crate::ui;
use crate::ui::detail;
use crate::ui::globals::*;
use crate::ui::new_backup;
use crate::ui::prelude::*;

pub fn main() {
    refresh();

    main_ui()
        .main_stack()
        .connect_property_visible_child_notify(|stack| {
            if let Some(page) = stack.get_visible_child_name() {
                match &page[..] {
                    "main" => {
                        main_ui().previous().hide();
                        main_ui().main_menu().show();
                        main_ui().detail_menu().hide();
                        main_ui().backup_run().hide();
                        refresh();
                    }
                    "archives" => {
                        main_ui().previous().show();
                        main_ui().main_menu().hide();
                        main_ui().detail_menu().hide();
                        main_ui().backup_run().hide();
                    }
                    _ => {
                        main_ui().previous().show();
                        main_ui().main_menu().hide();
                        main_ui().detail_menu().show();
                        main_ui().backup_run().show();
                    }
                }
            }
        });

    main_ui().previous().connect_clicked(|_| {
        if let Some(page) = main_ui().main_stack().get_visible_child_name() {
            match &page[..] {
                "archives" => {
                    main_ui().main_stack().set_visible_child_name("backup_conf");
                }
                _ => {
                    main_ui().main_stack().set_visible_child_name("main");
                }
            }
        }
    });

    main_ui().main_backups().connect_row_activated(|_, row| {
        let name = row.get_widget_name();
        if name == "-add" {
            new_backup::new_backup()
        } else {
            detail::view_backup_conf(&name)
        }
    });
}

pub fn refresh() {
    let list = main_ui().main_backups();
    ui::utils::clear(&list);

    if SETTINGS.load().backups.is_empty() {
        main_ui().overview_none_empty().hide();
        main_ui().overview_empty().show();
    } else {
        main_ui().overview_none_empty().show();
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
