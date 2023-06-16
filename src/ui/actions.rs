use crate::ui;
use crate::ui::prelude::*;
use adw::prelude::*;

pub fn init() {
    let action = crate::action::backup_show();
    action.connect_activate(|_, config_id| {
        if let Some(config_id) = config_id.and_then(|v| v.str()) {
            ui::page_backup::view_backup_conf(&ConfigId::new(config_id.to_string()));
            adw_app().activate();
        }
    });
    adw_app().add_action(&action);

    let action = crate::action::backup_start();
    action.connect_activate(|_, config_id| {
        info!("action backup.start: called");
        if let Some(config_id) = config_id.and_then(|v| v.str()) {
            ui::page_backup::activate_action_backup(ConfigId::new(config_id.to_string()));
        } else {
            error!("action backup.start: Did not receivce valid config id");
        }
    });
    adw_app().add_action(&action);

    let action = gio::SimpleAction::new("about", None);
    action.connect_activate(|_, _| ui::dialog_about::show());
    adw_app().add_action(&action);

    let action = gio::SimpleAction::new("setup", None);
    action.connect_activate(|_, _| ui::dialog_setup::show());
    adw_app().add_action(&action);

    let action = gio::SimpleAction::new("help", None);
    action.connect_activate(|_, _| {
        gtk::show_uri(
            Some(&main_ui().window()),
            "help:pika-backup",
            gtk::gdk::CURRENT_TIME,
        );
    });
    adw_app().add_action(&action);

    let action = gio::SimpleAction::new("quit", None);
    action.connect_activate(|_, _| {
        debug!("Potential quit: Action app.quit (Ctrl+Q)");
        Handler::run(ui::quit());
    });
    adw_app().add_action(&action);

    let action = gio::SimpleAction::new("backup-preferences", None);
    action.connect_activate(|_, _| {
        if let Some(id) = &**ui::ACTIVE_BACKUP_ID.load() {
            if ui::page_detail::is_navigation_page_visible() {
                // Only display when the backup detail page is open
                ui::dialog_preferences::DialogPreferences::new(id.clone()).show();
            }
        }
    });
    adw_app().add_action(&action);

    let action = gio::SimpleAction::new("remove", None);
    action.connect_activate(|_, _| ui::page_overview::remove_backup());
    adw_app().add_action(&action);
}
