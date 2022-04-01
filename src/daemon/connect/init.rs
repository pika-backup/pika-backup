use gio::prelude::*;

thread_local!(
    static VOLUME_MONITOR: gio::VolumeMonitor = gio::VolumeMonitor::get();
);

pub fn init() {
    init_device_monitor();
}

fn init_device_monitor() {
    VOLUME_MONITOR.with(|volume_monitor| {
        volume_monitor.connect_volume_added(|_, volume| {
            super::event::volume_added(volume);
        });

        volume_monitor.connect_volume_removed(|_, volume| {
            super::event::volume_removed(volume);
        });
    });
}
