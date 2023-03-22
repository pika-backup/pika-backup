use gio::prelude::*;
use once_cell::unsync::OnceCell;

use super::action;
use crate::config;
use crate::config::{ConfigType, Loadable, TrackChanges};
use crate::daemon;
use crate::daemon::prelude::*;

pub fn init() {
    gio_app().connect_startup(on_startup);
}

thread_local! {
static HOLD: OnceCell<ApplicationHoldGuard> = OnceCell::default();
}

fn on_startup(_app: &gio::Application) {
    HOLD.with(|hold| hold.set(gio_app().hold()).unwrap());

    crate::utils::init_gettext();

    config::Histories::update_on_change(&BACKUP_HISTORY).handle("Initial config load failed");
    config::Backups::update_on_change(&BACKUP_CONFIG).handle("Initial config load failed");

    daemon::connect::init::init();
    daemon::schedule::init::init();

    gio_app().add_action(&action::Quit::action());
    gio_app().add_action(&action::StartBackup::action());
    gio_app().add_action(&action::ShowOverview::action());
    gio_app().add_action(&action::ShowSchedule::action());

    glib::MainContext::default().spawn(async {
        match ashpd::desktop::background::BackgroundProxy::new().await {
            Ok(proxy) => {
                if let Err(err) = proxy
                    .set_status(&gettext("Monitoring Backup Schedule"))
                    .await
                {
                    error!("Error setting background status: {err:?}");
                }
            }
            Err(err) => error!("Error acquiring background proxy: {err:?}"),
        }

        crate::utils::listen_remote_app_running(crate::APP_ID, app_running)
            .await
            .handle("Cannot monitor ui status.")
    });
}

fn app_running(is_running: bool) {
    if !is_running {
        // Reload backup history manually to prevent race conditions between the application exit event and file monitor
        match config::Histories::from_file() {
            Ok(new) => BACKUP_HISTORY.update(|s| *s = new.clone()),
            Err(err) => {
                error!("Failed to reload {:?}: {}", config::Histories::path(), err);
            }
        }

        let backups_running = BACKUP_HISTORY
            .load()
            .iter()
            .filter(|(_, x)| x.running.is_some())
            .count();
        if backups_running > 0 {
            let notification = gio::Notification::new(&gettext("Fatal Error During Back Up"));

            notification.set_body(Some(&ngettextf_(
                "Pika Backup crashed while running a backup.",
                "Pika Backup crashed while running {} backups.",
                backups_running as u32,
            )));

            notification.set_default_action(&action::ShowOverview::name());

            gio_app().send_notification(None, &notification);
        }
    }
}
