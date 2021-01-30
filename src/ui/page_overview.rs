use gtk::prelude::*;
use libhandy::prelude::*;

use std::collections::HashMap;
use std::sync::RwLock;

use crate::config;
use crate::ui;
use crate::ui::globals::*;
use crate::ui::prelude::*;

thread_local!(
    static ROWS: RwLock<HashMap<ConfigId, ui::builder::OverviewItem>> =
        RwLock::new(Default::default());
);

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
        .add_backup()
        .connect_clicked(|_| ui::dialog_add_config::new_backup());
    main_ui()
        .add_backup_empty()
        .connect_clicked(|_| ui::dialog_add_config::new_backup());

    main_ui()
        .remove_backup()
        .connect_clicked(|_| spawn_local(on_remove_backup()));

    glib::timeout_add_seconds_local(1, || {
        if is_visible() {
            refresh_status();
        }
        Continue(true)
    });
}

fn is_visible() -> bool {
    main_ui().main_stack().get_visible_child()
        == Some(main_ui().page_overview().upcast::<gtk::Widget>())
}

fn on_main_stack_changed(_stack: &gtk::Stack) {
    if is_visible() {
        refresh();
    }
}

async fn on_remove_backup() {
    let delete = ui::utils::confirmation_dialog(
        &gettext("Delete backup configuration?"),
        &gettext("Deleting the configuration will not delete your saved data."),
        &gettext("Cancel"),
        &gettext("Delete"),
    )
    .await;

    if delete {
        if let Some(config) = SETTINGS.get().backups.get_active() {
            SETTINGS.update(move |s| {
                s.backups.remove(&config.id);
            });

            ui::utils::dialog_catch_err(
                ui::utils::secret_service_delete_passwords(&config.id),
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
    ROWS.with(|rows| {
        let _lock_error = rows.write().map(|mut x| (*x).clear());
    });

    for config in SETTINGS.load().backups.values() {
        let list_row = libhandy::ActionRowBuilder::new().activatable(true).build();
        list.add(&list_row);

        list_row.connect_activated(enclose!((config) move |_| {
            ui::page_detail::view_backup_conf(&config.id);
        }));

        let row = ui::builder::OverviewItem::new();
        list_row.add_prefix(&row.widget());

        row.status_spinner().connect_map(|s| s.start());
        row.status_spinner().connect_unmap(|s| s.stop());

        // Repo Icon

        if let Ok(icon) = gio::Icon::new_for_string(&config.repo.icon_symbolic()) {
            row.location_icon()
                .set_from_gicon(&icon, gtk::IconSize::Button);
        }

        // Repo Name

        let mut location = String::new();

        if let config::BackupRepo::Local { mount_name, .. } = &config.repo {
            location = format!(
                "{} – {}",
                mount_name.as_ref().map(|x| x.as_str()).unwrap_or_default(),
                config.repo.get_subtitle(),
            )
        } else {
            location.push_str(&config.repo.to_string());
        }

        row.location().set_text(&location);

        // Include

        for path in &config.include {
            let incl = gtk::BoxBuilder::new()
                .orientation(gtk::Orientation::Horizontal)
                .spacing(6)
                .build();
            row.include().add(&incl);

            incl.add_css_class("backup-include");

            if let Some(icon) =
                ui::utils::file_symbolic_icon(&config::absolute(path), gtk::IconSize::Button)
            {
                incl.add(&icon);
            }

            let path_str = if path.iter().next().is_none() {
                gettext("Home")
            } else {
                path.to_string_lossy().to_string()
            };

            let label = gtk::LabelBuilder::new()
                .label(&path_str)
                .ellipsize(pango::EllipsizeMode::Middle)
                .build();
            incl.add(&label);
        }

        ROWS.with(|rows| {
            let _lock_error = rows
                .write()
                .map(move |mut x| (*x).insert(config.id.clone(), row));
        });
    }

    refresh_status();
    list.show_all();
}

pub fn refresh_status() {
    for config in SETTINGS.load().backups.values() {
        ROWS.with(|rows| {
            if let Ok(rows) = rows.try_read() {
                if let Some(row) = rows.get(&config.id) {
                    let status = ui::backup_status::Display::new_from_id(&config.id);

                    row.status().set_text(&status.title);

                    if let Some(subtitle) = status.subtitle {
                        row.substatus().set_text(&subtitle);
                        row.substatus().show();
                    } else {
                        row.substatus().hide();
                    }

                    match &status.graphic {
                        ui::backup_status::Graphic::Icon(icon) => {
                            row.status_area().remove_css_class("error");
                            row.status_area().add_css_class("dim-label");

                            row.status_icon()
                                .set_from_icon_name(Some(icon), gtk::IconSize::Button);
                            row.status_graphic().set_visible_child(&row.status_icon());
                        }
                        ui::backup_status::Graphic::ErrorIcon(icon) => {
                            row.status_area().add_css_class("error");
                            row.status_area().remove_css_class("dim-label");

                            row.status_icon()
                                .set_from_icon_name(Some(icon), gtk::IconSize::Button);
                            row.status_graphic().set_visible_child(&row.status_icon());
                        }
                        ui::backup_status::Graphic::Spinner => {
                            row.status_area().remove_css_class("error");
                            row.status_area().add_css_class("dim-label");

                            row.status_graphic()
                                .set_visible_child(&row.status_spinner());
                        }
                    }
                }
            }
        });
    }
}
