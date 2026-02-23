use common::utils::action::Action;
use gio::prelude::*;

use super::dbus;
use super::prelude::*;

pub struct Restart;

impl Action<()> for Restart {
    const NAME: &'static str = "restart";

    fn activate(_: ()) {
        tracing::debug!("Restarting the daemon via dbus restart action");
        glib::MainContext::default().block_on(crate::init::restart_daemon());
    }
}

pub struct Quit;

impl Action<()> for Quit {
    const NAME: &'static str = "quit";

    fn activate(_: ()) {
        tracing::debug!("Quitting the daemon via dbus quit action");
        if let Some(app) = gio::Application::default() {
            app.quit();
        }
    }
}

pub struct StartBackup;

impl Action<String> for StartBackup {
    const NAME: &'static str = "start-backup";
    const PARAMETER_TYPE: Option<&glib::VariantTy> = Some(glib::VariantTy::STRING);

    fn activate(config_id: String) {
        glib::MainContext::default().spawn(async move {
            dbus::PikaBackup::start_backup(&ConfigId::new(config_id))
                .await
                .handle(gettext("Failed to start backup from daemon"));
        });
    }
}

pub struct ShowOverview;

impl Action<()> for ShowOverview {
    const NAME: &'static str = "show-overview";

    fn activate(_: ()) {
        glib::MainContext::default().spawn(async move {
            dbus::PikaBackup::show_overview()
                .await
                .handle(gettext("Failed to show overview from daemon"));
        });
    }
}

pub struct ShowSchedule;

impl Action<String> for ShowSchedule {
    const NAME: &'static str = "show-schedule";
    const PARAMETER_TYPE: Option<&glib::VariantTy> = Some(glib::VariantTy::STRING);

    fn activate(config_id: String) {
        glib::MainContext::default().spawn(async move {
            dbus::PikaBackup::show_schedule(&ConfigId::new(config_id))
                .await
                .handle(gettext("Failed to show schedule from daemon"));
        });
    }
}
