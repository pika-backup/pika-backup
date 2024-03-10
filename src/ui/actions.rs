use crate::ui;
use crate::ui::prelude::*;
use adw::prelude::*;

pub fn init() {
    let action = crate::action::backup_show();
    action.connect_activate(|_, config_id| {
        if let Some(config_id) = config_id.and_then(|v| v.str()) {
            main_ui()
                .page_detail()
                .backup_page()
                .view_backup_conf(&ConfigId::new(config_id.to_string()));
            adw_app().activate();
        }
    });
    adw_app().add_action(&action);

    let action = crate::action::backup_start();
    action.connect_activate(|_, config_id| {
        info!("action backup.start: called");
        if let Some(config_id) = config_id.and_then(|v| v.str()).map(ToString::to_string) {
            let guard = QuitGuard::default();
            main_ui().page_detail().backup_page().start_backup(
                ConfigId::new(config_id),
                None,
                guard,
            );
        } else {
            error!("action backup.start: Did not receive valid config id");
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
    let context = adw_app()
        .active_window()
        .map(|w| gtk::prelude::WidgetExt::display(&w).app_launch_context());

    action.connect_activate(move |_, _| {
        if let Err(err) = gio::AppInfo::launch_default_for_uri("help:pika-backup", context.as_ref())
        {
            error!("Launch help error: {err}");
        }
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
            if main_ui().page_detail().is_visible() {
                // Only display when the backup detail page is open
                ui::dialog_preferences::DialogPreferences::new(id.clone()).present();
            }
        }
    });
    adw_app().add_action(&action);

    let action = gio::SimpleAction::new("remove", None);
    action.connect_activate(|_, _| main_ui().page_overview().remove_backup());
    adw_app().add_action(&action);
}
