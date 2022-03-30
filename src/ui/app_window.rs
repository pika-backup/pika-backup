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

    gtk_app().add_window(&main_ui().window());

    Handler::handle(load_resources());
}

pub fn is_displayed() -> bool {
    main_ui().window().is_visible()
}

pub fn show() {
    if !is_displayed() {
        debug!("Displaying ui that was hidden before.");

        main_ui().window().present();

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
                        if let Some(config) =
                            BACKUP_CONFIG.load().get_result(config_id).ok().cloned()
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

fn on_delete() -> gtk::Inhibit {
    debug!("Potential quit: ApplicationWindow delete event");

    Handler::run(super::quit());
    gtk::Inhibit(true)
}

#[cfg(not(debug_assertions))]
fn resource() -> std::result::Result<gio::Resource, glib::Error> {
    gio::Resource::from_data(&glib::Bytes::from_static(include_bytes!(env!(
        "G_RESOURCES_PATH"
    ))))
}

#[cfg(debug_assertions)]
fn resource() -> std::result::Result<gio::Resource, glib::Error> {
    if let Some(path) = option_env!("G_RESOURCES_PATH") {
        gio::Resource::load(&path)
    } else {
        gio::Resource::load("data/resources.gresource")
    }
}

fn load_resources() -> Result<()> {
    let res = resource().err_to_msg(gettext("Failed to load app assets."))?;

    gio::resources_register(&res);
    Ok(())
}
