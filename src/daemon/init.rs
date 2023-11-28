use gio::prelude::*;
use once_cell::unsync::OnceCell;
use std::cell::Cell;
use std::collections::HashMap;
use std::path::PathBuf;

use super::action;
use crate::config::{ConfigType, Loadable, TrackChanges};
use crate::daemon;
use crate::daemon::prelude::*;
use crate::{config, DAEMON_BINARY};

pub fn init() {
    gio_app().connect_startup(on_startup);
}

thread_local! {
    static HOLD: OnceCell<ApplicationHoldGuard> = OnceCell::default();
    static FILE_MONITOR_FLATPAK_UPDATED: OnceCell<gio::FileMonitor> = OnceCell::default();
    static APP_RUNNING: Cell<bool> = Cell::default();
}

fn on_startup(_app: &gio::Application) {
    HOLD.with(|hold| hold.set(gio_app().hold()).unwrap());

    crate::utils::init_gettext();

    let config_load_result =
        config::Histories::update_on_change(&BACKUP_HISTORY, config_reload_error_handler).and_then(
            |_| config::Backups::update_on_change(&BACKUP_CONFIG, config_reload_error_handler),
        );

    if let Err(err) = &config_load_result {
        let msg = gettext("Error loading configuration");
        let detail = format!("{}\n{}", gettext("Not monitoring backup schedule."), err);
        error!("Error loading configuration: {}: {}", msg, detail);

        let notification = gio::Notification::new(&msg.to_string());
        notification.set_body(Some(&detail));
        gio_app().send_notification(None, &notification);

        // If we can't read the config, quit the monitor process
        gio_app().quit();
        return;
    }

    daemon::connect::init::init();
    daemon::schedule::init::init();

    gio_app().add_action(&action::Restart::action());
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

    if *APP_IS_SANDBOXED {
        // Register a file monitor to check for the /app/.updated file
        // Creation of this file signifies that the app had been updated
        let file = gio::File::for_path("/app/.updated");

        if let Ok(monitor) = file.monitor_file(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE)
        {
            monitor.connect_changed(|_monitor, _file, _other, event| {
                if event == gio::FileMonitorEvent::Created {
                    if APP_RUNNING.get() {
                        info!("Detected flatpak update. Not restarting monitor process, main app is still running.");
                    } else {
                        info!("Detected flatpak update. Restarting monitor process.");
                        glib::MainContext::default().block_on(restart_daemon());
                    }
                }
            });

            FILE_MONITOR_FLATPAK_UPDATED
                .with(|m| m.set(monitor))
                .unwrap();
        }
    }
}

fn app_running(is_running: bool) {
    APP_RUNNING.set(is_running);

    if !is_running {
        // Reload backup history manually to prevent race conditions between the application exit event and file monitor
        match config::Histories::from_file() {
            Ok(new) => {
                BACKUP_HISTORY.swap(Arc::new(new));
            }
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

        // Detect app update
        if *APP_IS_SANDBOXED && PathBuf::from("/app/.updated").exists() {
            info!("Detected flatpak update. Restarting monitor process.");
            glib::MainContext::default().spawn(restart_daemon());
        }
    }
}

pub fn config_reload_error_handler(err: std::io::Error) {
    warn!("Error reloading config: {}. Restarting daemon.", err);
    glib::MainContext::default().spawn(restart_daemon());
}

pub async fn restart_daemon() {
    if *APP_IS_SANDBOXED {
        let flatpak_result = ashpd::flatpak::Flatpak::new().await;
        if let Ok(flatpak) = flatpak_result {
            let binary = PathBuf::from("/app/bin/").join(DAEMON_BINARY);

            flatpak
                .spawn(
                    glib::current_dir(),
                    &[binary.as_os_str(), "--gapplication-replace".as_ref()],
                    HashMap::new(),
                    HashMap::new(),
                    ashpd::flatpak::SpawnFlags::LatestVersion.into(),
                    ashpd::flatpak::SpawnOptions::default(),
                )
                .await
                .handle(gettext("Error restarting monitor daemon"));
        } else {
            flatpak_result.handle(gettext("Error restarting monitor daemon"));
        }
    } else {
        let mut command = async_std::process::Command::new(DAEMON_BINARY);
        command.arg("--gapplication-replace");
        command
            .spawn()
            .handle(gettext("Error restarting monitor daemon"));
    }
}
