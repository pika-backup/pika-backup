use crate::ui::prelude::*;
use adw::prelude::*;

use std::collections::BTreeMap;
use std::sync::RwLock;

use crate::ui;

pub fn dbus_show() {
    main_ui()
        .main_stack()
        .set_visible_child(&main_ui().page_overview());
    adw_app().activate();
}

pub fn refresh_status() {
    if is_visible() {
        force_refresh_status();
    }
}

thread_local!(
    static ROWS: RwLock<BTreeMap<ConfigId, ui::builder::OverviewItem>> =
        RwLock::new(Default::default());
);

pub fn init() {
    main_ui()
        .add_backup()
        .connect_clicked(|_| ui::dialog_setup::show());
    main_ui()
        .add_backup_empty()
        .connect_clicked(|_| ui::dialog_setup::show());

    main_ui().main_backups().connect_map(|_| rebuild_list());
    reload_visible_page();
}

fn is_visible() -> bool {
    main_ui().main_stack().visible_child()
        == Some(main_ui().page_overview().upcast::<gtk::Widget>())
}

pub fn remove_backup() {
    Handler::run(on_remove_backup());
}

async fn on_remove_backup() -> Result<()> {
    ui::utils::confirmation_dialog(
        &gettext("Remove Backup Setup?"),
        &gettext("Removing the setup will not delete any archives."),
        &gettext("Cancel"),
        &gettext("Remove Setup"),
    )
    .await?;

    let config = BACKUP_CONFIG.load().active()?.clone();

    let config_id = config.id.clone();

    BACKUP_CONFIG.try_update(|s| {
        s.remove(&config_id)?;
        Ok(())
    })?;

    if let Err(err) = ui::utils::password_storage::remove_password(&config, false).await {
        // Display the error and continue to leave the UI in a consistent state
        err.show().await;
    }

    ACTIVE_BACKUP_ID.update(|active_id| *active_id = None);

    reload_visible_page();
    main_ui()
        .navigation_view()
        .pop_to_page(&main_ui().navigation_page_overview());

    Ok(())
}

pub fn reload_visible_page() {
    if BACKUP_CONFIG.load().iter().next().is_none() {
        main_ui()
            .main_stack()
            .set_visible_child(&main_ui().page_overview_empty());
    } else {
        main_ui()
            .main_stack()
            .set_visible_child(&main_ui().page_overview());
    };
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

        row.location_title().set_label(&config.title());
        row.location_subtitle().set_label(&config.repo.subtitle());

        // Include

        for path in &config.include {
            let incl = ui::widget::LocationTag::from_path(path.clone());

            row.include().add_child(&incl.build());
        }

        ROWS.with(|rows| {
            let _lock_error = rows
                .write()
                .map(move |mut x| (*x).insert(config.id.clone(), row));
        });
    }

    force_refresh_status();
}

fn force_refresh_status() {
    glib::MainContext::default().spawn_local(async move {
        for config in BACKUP_CONFIG.load().iter() {
            let schedule_status = ui::page_schedule::status::Status::new(config).await;
            ROWS.with(move |rows| {
                if let Ok(rows) = rows.try_read() {
                    if let Some(row) = rows.get(&config.id) {
                        let status = ui::backup_status::Display::new_from_id(&config.id);

                        row.status().set_from_backup_status(&status);
                        // schedule status

                        row.schedule()
                            .set_title(&glib::markup_escape_text(&schedule_status.main.title()));
                        row.schedule().set_subtitle(&glib::markup_escape_text(
                            &schedule_status.main.subtitle().unwrap_or_default(),
                        ));
                        row.schedule()
                            .set_icon_name(schedule_status.main.icon_name());
                        row.schedule().set_level(schedule_status.main.level());
                    }
                }
            })
        }
    });
}
