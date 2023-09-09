use adw::prelude::*;
use async_std::prelude::*;
use ui::prelude::*;

use crate::ui;
use glib::SignalHandlerId;
use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// Forced UI updates to update 'time ago' etc.
const UI_INTERVAL: Duration = Duration::from_secs(60);

pub struct StatusTracking {
    pub on_battery_since: Cell<Option<Instant>>,
    pub metered_since: Cell<Option<Instant>>,
    pub daemon_running: Cell<bool>,
    metered_signal_handler: Cell<Option<SignalHandlerId>>,
    volume_monitor: Cell<Option<gio::VolumeMonitor>>,
    quit_inhibit_count: Cell<usize>,
}

impl StatusTracking {
    pub fn new_rc() -> Rc<Self> {
        debug!("Setting up global status tracking");

        let tracking = Rc::new(Self {
            on_battery_since: Default::default(),
            metered_since: Default::default(),
            daemon_running: Default::default(),
            metered_signal_handler: Default::default(),
            volume_monitor: Default::default(),
            quit_inhibit_count: Default::default(),
        });

        // Metered
        tracking.metered_signal_handler.set(Some(
            gio::NetworkMonitor::default().connect_network_metered_notify(
                glib::clone!(@weak tracking => move |x| {
                    if x.is_network_metered() {
                        debug!("Connection now metered.");
                        tracking.metered_since.set(Some(Instant::now()));
                    } else {
                        debug!("Connection no longer metered.");
                        tracking.metered_since.set(None);
                    }
                    tracking.ui_schedule_update();
                }),
            ),
        ));

        // Battery
        let weak_tracking = Rc::downgrade(&tracking);
        glib::MainContext::default().spawn_local(async move {
            if let Some(mut stream) =
                crate::utils::upower::UPower::receive_on_battery_changed().await
            {
                while let (Some(result), Some(tracking)) =
                    (stream.next().await, weak_tracking.upgrade())
                {
                    match result.get().await {
                        Ok(true) => {
                            debug!("Device now battery powered.");
                            tracking.on_battery_since.set(Some(Instant::now()));
                        }
                        Ok(false) => {
                            debug!("Device no longer battery powered.");
                            tracking.on_battery_since.set(None);
                        }
                        Err(err) => {
                            warn!("Failed to get new OnBattery() status: {}", err);
                        }
                    }
                    tracking.ui_schedule_update();
                }
            }
        });

        // Daemon
        Handler::run(enclose!((tracking) async {
            crate::utils::listen_remote_app_running(crate::DAEMON_APP_ID, move |running| {
                tracking.daemon_running.set(running);
                tracking.ui_schedule_update();
            })
            .await
            .err_to_msg(gettext("Unable to determine background process status"))
        }));

        // Volume/Mount
        let volume_monitor = gio::VolumeMonitor::get();
        tracking.volume_monitor.set(Some(volume_monitor.clone()));

        volume_monitor.connect_volume_added(enclose!((tracking) move |_, _| {
            tracking.ui_schedule_update();
            ui::page_backup::refresh_disk_status();
        }));

        volume_monitor.connect_volume_removed(enclose!((tracking) move |_, _| {
            tracking.ui_schedule_update();
            ui::page_backup::refresh_disk_status();
        }));

        volume_monitor.connect_mount_added(move |_, _| {
            ui::page_backup::refresh_disk_status();
        });

        volume_monitor.connect_mount_removed(move |_, _| {
            ui::page_backup::refresh_disk_status();
        });

        // Regular update
        glib::source::timeout_add_local(
            UI_INTERVAL,
            glib::clone!(@weak tracking => @default-return glib::ControlFlow::Break, move || {
                debug!("Regular UI update to keep 'time ago' etc correct.");
                tracking.ui_status_update();
                tracking.ui_schedule_update();
                glib::ControlFlow::Continue
            }),
        );

        tracking
    }

    pub fn quit_inhibit_count(&self) -> usize {
        self.quit_inhibit_count.get()
    }

    fn ui_status_update(&self) {
        debug!("UI status update");

        ui::page_backup::refresh_status();
        ui::page_archives::refresh_status();
        ui::page_overview::refresh_status();
        ui::dialog_info::refresh_status();
    }

    fn ui_schedule_update(&self) {
        debug!("UI schedule update");

        ui::page_schedule::refresh_status();
        ui::page_overview::refresh_status();
    }
}

impl Drop for StatusTracking {
    fn drop(&mut self) {
        debug!("Dropping global status tracking");
    }
}

#[non_exhaustive]
pub struct QuitGuard;

impl Default for QuitGuard {
    /// Create a quit guard that will quit the app if no other guards are running at the same time
    fn default() -> Self {
        ui::globals::STATUS_TRACKING.with(|status| {
            let new = status.quit_inhibit_count.get() + 1;
            debug!("Increasing quit guard count to {new}");
            status.quit_inhibit_count.set(new);
        });

        Self {}
    }
}

impl Drop for QuitGuard {
    fn drop(&mut self) {
        let mut quit = false;

        ui::globals::STATUS_TRACKING.with(|status| {
            let new = std::cmp::max(status.quit_inhibit_count.get(), 1) - 1;
            debug!("Decreasing quit guard count to {new}");
            status.quit_inhibit_count.set(new);
            quit = new == 0;

            status.clone()
        });

        // Don't quit the app when testing
        #[cfg(not(test))]
        if quit && !**IS_SHUTDOWN.load() {
            // Checks whether window is open and quits if necessary
            glib::MainContext::default().spawn_from_within(|| async {
                if !main_ui().window().is_visible() {
                    let _ = ui::quit().await;
                }
            });
        }
    }
}
