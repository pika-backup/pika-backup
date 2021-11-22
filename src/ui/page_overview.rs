use adw::prelude::*;

use std::collections::BTreeMap;
use std::sync::RwLock;

use crate::config;
use crate::ui;
use crate::ui::prelude::*;

thread_local!(
    static ROWS: RwLock<BTreeMap<ConfigId, ui::builder::OverviewItem>> =
        RwLock::new(Default::default());
);

pub fn init() {
    if BACKUP_CONFIG.load().iter().count() > 1 {
        main_ui()
            .main_stack()
            .set_visible_child(&main_ui().page_overview());
        rebuild_list();
    } else if let Some(config) = BACKUP_CONFIG.load().iter().next() {
        ui::page_backup::view_backup_conf(&config.id);
    } else {
        main_ui()
            .main_stack()
            .set_visible_child(&main_ui().page_overview_empty());
    }

    main_ui()
        .main_stack()
        .connect_visible_child_notify(on_main_stack_changed);

    main_ui()
        .add_backup()
        .connect_clicked(|_| ui::dialog_add_config::new_backup());
    main_ui()
        .add_backup_empty()
        .connect_clicked(|_| ui::dialog_add_config::new_backup());

    glib::timeout_add_seconds_local(1, || {
        if is_visible() {
            refresh_status();
        }
        Continue(true)
    });
}

fn is_visible() -> bool {
    main_ui().main_stack().visible_child()
        == Some(main_ui().page_overview().upcast::<gtk::Widget>())
}

fn on_main_stack_changed(_stack: &adw::ViewStack) {
    if is_visible() {
        rebuild_list();
    }
}

pub fn remove_backup() {
    Handler::run(on_remove_backup());
}

async fn on_remove_backup() -> Result<()> {
    ui::utils::confirmation_dialog(
        &gettext("Delete backup configuration?"),
        &gettext("Deleting the configuration will not delete your saved data."),
        &gettext("Cancel"),
        &gettext("Delete"),
    )
    .await?;

    let config_id = BACKUP_CONFIG.get().active()?.id.clone();

    BACKUP_CONFIG.update_result(|s| {
        s.remove(&config_id)?;
        Ok(())
    })?;

    ui::utils::secret_service::delete_passwords(&config_id).err_to_msg(gettext(
        "Failed to remove potentially remaining passwords from key storage.",
    ))?;

    ACTIVE_BACKUP_ID.update(|active_id| *active_id = None);
    ui::write_config()?;

    if BACKUP_CONFIG.load().iter().next().is_none() {
        main_ui()
            .main_stack()
            .set_visible_child(&main_ui().page_overview_empty());
    } else {
        main_ui()
            .main_stack()
            .set_visible_child(&main_ui().page_overview());
    };

    Ok(())
}

fn rebuild_list() {
    let list = main_ui().main_backups();

    ui::utils::clear(&list);

    ROWS.with(|rows| {
        let _lock_error = rows.write().map(|mut x| (*x).clear());
    });

    for config in BACKUP_CONFIG.load().iter() {
        let row = ui::builder::OverviewItem::new();
        list.append(&row.widget());

        row.status_spinner().connect_map(|s| s.start());
        row.status_spinner().connect_unmap(|s| s.stop());

        // connect click

        row.location()
            .connect_activated(enclose!((config) move |_| {
                ui::page_backup::view_backup_conf(&config.id);
            }));

        row.schedule()
            .connect_activated(enclose!((config) move |_| {
                ui::page_schedule::view(&config.id);
            }));

        // Repo Icon

        if let Ok(icon) = gio::Icon::for_string(&config.repo.icon()) {
            row.location_icon().set_from_gicon(&icon);
        }

        // Repo Name

        row.location_title().set_label(&config.repo.title());
        row.location_subtitle().set_label(&config.repo.subtitle());

        // Include

        for path in &config.include {
            let incl = gtk::Box::builder()
                .orientation(gtk::Orientation::Horizontal)
                .spacing(4)
                .build();
            row.include().append(&incl);

            incl.add_css_class("backup-include");

            if let Some(icon) = ui::utils::file_symbolic_icon(&config::absolute(path)) {
                incl.append(&icon);
            }

            let path_str = if path.iter().next().is_none() {
                gettext("Home")
            } else {
                path.to_string_lossy().to_string()
            };

            let label = gtk::Label::builder()
                .label(&path_str)
                .ellipsize(pango::EllipsizeMode::Middle)
                .build();
            incl.append(&label);
        }

        ROWS.with(|rows| {
            let _lock_error = rows
                .write()
                .map(move |mut x| (*x).insert(config.id.clone(), row));
        });
    }

    refresh_status();
}

pub fn refresh_status() {
    for config in BACKUP_CONFIG.load().iter() {
        ROWS.with(|rows| {
            if let Ok(rows) = rows.try_read() {
                if let Some(row) = rows.get(&config.id) {
                    let status = ui::backup_status::Display::new_from_id(&config.id);

                    row.status().set_title(&status.title);

                    row.status()
                        .set_subtitle(&status.subtitle.unwrap_or_default());

                    match &status.graphic {
                        ui::backup_status::Graphic::OkIcon(icon) => {
                            row.status_icon().set_ok();
                            row.status_icon().set_icon_name(icon);

                            row.status_graphic().set_visible_child(&row.status_icon());
                        }

                        ui::backup_status::Graphic::WarningIcon(icon) => {
                            row.status_icon().set_warning();
                            row.status_icon().set_icon_name(icon);

                            row.status_graphic().set_visible_child(&row.status_icon());
                        }
                        ui::backup_status::Graphic::ErrorIcon(icon) => {
                            row.status_icon().set_error();
                            row.status_icon().set_icon_name(icon);

                            row.status_graphic().set_visible_child(&row.status_icon());
                        }
                        ui::backup_status::Graphic::Spinner => {
                            row.status_graphic()
                                .set_visible_child(&row.status_spinner());
                        }
                    }

                    // schedule status

                    let status = ui::page_schedule::status::Status::new(config);

                    row.schedule().set_title(&status.main.title);
                    row.schedule().set_subtitle(&status.main.subtitle);
                    row.schedule_icon().set_icon_name(&status.main.icon_name);
                    row.schedule_icon().set_level(status.main.level);
                }
            }
        });
    }
}
