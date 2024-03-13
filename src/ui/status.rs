use adw::prelude::*;
use async_std::prelude::*;
use ui::prelude::*;

use crate::ui;
use glib::SignalHandlerId;
use std::cell::Cell;
use std::marker::PhantomData;
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
    idle_since: Cell<Option<Instant>>,
}

impl StatusTracking {
    pub fn new_rc() -> Rc<Self> {
        if !gtk::is_initialized_main_thread() {
            error!("StatusTracking must not be initialized outside of the main thread");
        }

        debug!("Setting up global status tracking");

        let tracking = Rc::new(Self {
            on_battery_since: Default::default(),
            metered_since: Default::default(),
            daemon_running: Default::default(),
            metered_signal_handler: Default::default(),
            volume_monitor: Default::default(),
            quit_inhibit_count: Default::default(),
            idle_since: Cell::new(Some(Instant::now())),
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
            crate::utils::listen_remote_app_running(
                crate::DAEMON_APP_ID,
                &crate::ui::dbus::session_connection()
                    .await
                    .err_to_msg(gettext("Unable to determine background process status"))?,
                move |running| {
                    tracking.daemon_running.set(running);
                    tracking.ui_schedule_update();
                },
            )
            .await
            .err_to_msg(gettext("Unable to determine background process status"))
        }));

        // Volume/Mount
        let volume_monitor = gio::VolumeMonitor::get();
        tracking.volume_monitor.set(Some(volume_monitor.clone()));

        volume_monitor.connect_volume_added(enclose!((tracking) move |_, _| {
            tracking.ui_schedule_update();
            main_ui().page_detail().backup_page().refresh_disk_status();
        }));

        volume_monitor.connect_volume_removed(enclose!((tracking) move |_, _| {
            tracking.ui_schedule_update();
            main_ui().page_detail().backup_page().refresh_disk_status();
        }));

        volume_monitor.connect_mount_added(move |_, _| {
            main_ui().page_detail().backup_page().refresh_disk_status();
        });

        volume_monitor.connect_mount_removed(move |_, _| {
            main_ui().page_detail().backup_page().refresh_disk_status();
        });

        // Regular update
        glib::source::timeout_add_local(
            UI_INTERVAL,
            glib::clone!(@weak tracking => @default-return glib::ControlFlow::Break, move || {
                // Check if UI is idle without task. This should usually not happen.
                if tracking.quit_inhibit_count() == 0 {
                    if let Some(idle_since) = tracking.idle_since.get() {
                        if idle_since.elapsed() > Duration::from_secs(120) && !main_ui().window().is_visible() {
                            error!("UI has been indle without task for 120 secs. Quitting.");
                            quit_background_app();
                        }
                    } else {
                        // Usually this should be set already
                        tracking.idle_since.set(Some(Instant::now()));
                    }
                }

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

        main_ui().page_detail().backup_page().refresh_status();
        main_ui().page_detail().archives_page().refresh_status();
        main_ui().page_overview().refresh_status();
    }

    fn ui_schedule_update(&self) {
        debug!("UI schedule update");

        main_ui().page_detail().schedule_page().refresh_status();
        main_ui().page_overview().refresh_status();
    }
}

impl Drop for StatusTracking {
    fn drop(&mut self) {
        debug!("Dropping global status tracking");
    }
}

pub struct QuitGuard(PhantomData<()>);

impl Default for QuitGuard {
    /// Create a quit guard that will quit the app if no other guards are running at the same time
    fn default() -> Self {
        // Invoke with higher priority than the Drop handler to make sure this runs first
        glib::MainContext::default().invoke_with_priority(glib::Priority::HIGH, || {
            ui::globals::STATUS_TRACKING.with(|status| {
                let new = status.quit_inhibit_count.get() + 1;
                debug!("Increasing quit guard count to {new}");
                status.quit_inhibit_count.set(new);
                status.idle_since.set(None);
            });
        });

        Self(PhantomData)
    }
}

impl Drop for QuitGuard {
    fn drop(&mut self) {
        glib::MainContext::default().invoke(|| {
            let new_count = ui::globals::STATUS_TRACKING.with(|status| {
                let count = status.quit_inhibit_count.get();

                if let Some(new) = count.checked_sub(1) {
                    debug!("Decreasing quit guard count to {new}");
                    status.quit_inhibit_count.set(new);

                    if new == 0 {
                        status.idle_since.set(Some(Instant::now()));
                    }
                } else {
                    error!("BUG: Would reduce quit guard to < 0. Something has gone terribly wrong with status tracking.");
                }

                status.quit_inhibit_count.get()
            });

            if new_count == 0 && !main_ui().window().is_visible() {
                quit_background_app();
            }
        });
    }
}

fn quit_background_app() {
    // Don't quit the app when testing
    #[cfg(not(test))]
    if !ui::App::default().in_shutdown() {
        // Checks whether window is open and quits if necessary
        glib::MainContext::default().spawn_from_within(|| async {
            let _ = ui::quit().await;
        });
    }
}
