use gtk::prelude::*;

use crate::borg;
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

    main_ui().internal_message().connect_close(|info_bar| {
        info_bar.set_revealed(false);
    });

    main_ui()
        .internal_message()
        .connect_response(|info_bar, _| {
            info_bar.set_revealed(false);
        });
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
        std::thread::spawn(|| {
            for (config_id, communication) in BACKUP_COMMUNICATION.load().iter() {
                if communication.status.load().estimated_size.is_none() {
                    debug!("A running backup is lacking size estimate");
                    if let Ok(config) = BACKUP_CONFIG.load().get_result(config_id) {
                        borg::size_estimate::recalculate(config, communication.clone());
                    }
                }
            }
        });
    } else {
        debug!("Not displaying ui because it is already visible.");
    }
}

fn on_delete() -> gtk::Inhibit {
    debug!("Potential quit: ApplicationWindow delete event");

    Handler::run(super::quit());
    gtk::Inhibit(true)
}
