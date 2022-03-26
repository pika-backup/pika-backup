use super::prelude::*;

use super::dbus;

pub trait Action {
    const NAME: &'static str;
    fn action() -> gio::SimpleAction;
    fn name() -> String {
        format!("app.{}", Self::NAME)
    }
}

pub struct StartBackup;

impl Action for StartBackup {
    const NAME: &'static str = "start-backup";

    fn action() -> gio::SimpleAction {
        let action = gio::SimpleAction::new(Self::NAME, Some(glib::VariantTy::STRING));
        action.connect_activate(|_, config_id| {
            if let Some(config_id) = config_id.and_then(|v| v.str().map(|x| x.to_string())) {
                glib::MainContext::default().spawn(async move {
                    dbus::PikaBackup::start_backup(&ConfigId::new(config_id))
                        .await
                        .handle(gettext("Failed to start backup from daemon"));
                });
            } else {
                error!("Invalid parameter for {}: {:?}", Self::NAME, config_id);
            }
        });
        action
    }
}

pub struct ShowOverview;

impl Action for ShowOverview {
    const NAME: &'static str = "show-overview";

    fn action() -> gio::SimpleAction {
        let action = gio::SimpleAction::new(Self::NAME, None);
        action.connect_activate(|_, _| {
            glib::MainContext::default().spawn(async move {
                dbus::PikaBackup::show_overview()
                    .await
                    .handle(gettext("Failed to show overview from daemon"));
            });
        });
        action
    }
}
