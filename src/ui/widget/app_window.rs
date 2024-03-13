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
        main_ui().window().add_css_class("devel");
    }

    main_ui()
        .navigation_view()
        .connect_visible_page_notify(|navigation_view| {
            if navigation_view
                .visible_page()
                .is_some_and(|page| page == main_ui().page_overview())
            {
                main_ui().page_overview().reload_visible_page();
            }
        });

    adw_app().add_window(&main_ui().window());
}

pub fn is_displayed() -> bool {
    main_ui().window().is_visible()
}

pub fn show() {
    let displayed = is_displayed();
    main_ui().window().present();

    if !displayed {
        debug!("Displaying ui that was hidden before.");

        Handler::run(ui::init_check_borg());

        // redo size estimates for backups running in background before
        BORG_OPERATION.with(|operations| {
            for (config_id, operation) in operations.load().iter() {
                if let Some(create_op) = operation.try_as_create() {
                    if create_op
                        .communication()
                        .specific_info
                        .load()
                        .estimated_size
                        .is_none()
                    {
                        debug!("A running backup is lacking size estimate");
                        if let Some(config) = BACKUP_CONFIG.load().try_get(config_id).ok().cloned()
                        {
                            let communication = create_op.communication().clone();
                            glib::MainContext::default().spawn_local(async move {
                                ui::toast_size_estimate::check(&config, communication).await
                            });
                        }
                    }
                }
            }
        });
    } else {
        debug!("Not displaying ui because it is already visible.");
    }
}

fn on_delete() -> glib::Propagation {
    debug!("Potential quit: ApplicationWindow delete event");

    Handler::run(crate::ui::quit());
    glib::Propagation::Stop
}
