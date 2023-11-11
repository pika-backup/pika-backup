use super::prelude::*;
use gio::prelude::*;

use super::dbus;

pub trait Action {
    const NAME: &'static str;
    fn action() -> gio::SimpleAction;
    fn name() -> String {
        format!("app.{}", Self::NAME)
    }
}

pub struct Restart;

impl Action for Restart {
    const NAME: &'static str = "restart";

    fn action() -> gio::SimpleAction {
        let action = gio::SimpleAction::new("restart", None);
        action.connect_activate(|_, _| {
            debug!("Restarting the daemon via dbus restart action");
            glib::MainContext::default().block_on(crate::daemon::init::restart_daemon());
        });
        action
    }
}

pub struct Quit;

impl Action for Quit {
    const NAME: &'static str = "quit";

    fn action() -> gio::SimpleAction {
        let action = gio::SimpleAction::new("quit", None);
        action.connect_activate(|_, _| {
            debug!("Quitting the daemon via dbus quit action");
            if let Some(app) = gio::Application::default() {
                app.quit();
            }
        });
        action
    }
}

pub struct StartBackup;

impl Action for StartBackup {
    const NAME: &'static str = "start-backup";

    fn action() -> gio::SimpleAction {
        let action = gio::SimpleAction::new(Self::NAME, Some(glib::VariantTy::STRING));
        action.connect_activate(|_, config_id| {
            if let Some(config_id) = config_id.and_then(glib::FromVariant::from_variant) {
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

pub struct ShowSchedule;

impl Action for ShowSchedule {
    const NAME: &'static str = "show-schedule";

    fn action() -> gio::SimpleAction {
        let action = gio::SimpleAction::new(Self::NAME, Some(glib::VariantTy::STRING));
        action.connect_activate(|_, config_id| {
            if let Some(config_id) = config_id.and_then(glib::FromVariant::from_variant) {
                glib::MainContext::default().spawn(async move {
                    dbus::PikaBackup::show_schedule(&ConfigId::new(config_id))
                        .await
                        .handle(gettext("Failed to show schedule from daemon"));
                });
            } else {
                error!("Invalid parameter for {}: {:?}", Self::NAME, config_id);
            }
        });
        action
    }
}
