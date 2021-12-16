use gtk::prelude::*;

use crate::ui;
use crate::ui::prelude::*;

pub fn init() {
    main_ui().window().connect_close_request(|_| on_delete());

    // decorate headerbar of pre-release versions
    if !option_env!("APPLICATION_ID_SUFFIX")
        .unwrap_or_default()
        .is_empty()
    {
        main_ui().window().style_context().add_class("devel");
    }
}

pub fn is_displayed() -> bool {
    main_ui().window().is_visible()
}

pub fn show() {
    if !is_displayed() {
        debug!("Displaying ui that was hidden before.");

        gtk_app().add_window(&main_ui().window());
        main_ui().window().present();

        Handler::run(ui::init_check_borg());

        // redo size estimates for backups running in background before
        for (config_id, communication) in BACKUP_COMMUNICATION.load().iter() {
            if communication.status.load().estimated_size.is_none() {
                debug!("A running backup is lacking size estimate");
                if let Some(config) = BACKUP_CONFIG.load().get_result(config_id).ok().cloned() {
                    let communication = communication.clone();
                    glib::MainContext::default().spawn_local(async move {
                        ui::toast_size_estimate::check(&config, communication).await
                    });
                }
            }
        }
    } else {
        debug!("Not displaying ui because it is already visible.");
    }
}

fn on_delete() -> gtk::Inhibit {
    debug!("Potential quit: ApplicationWindow delete event");

    Handler::run(super::quit());
    gtk::Inhibit(true)
}
