use gio::prelude::*;

thread_local!(
    static VOLUME_MONITOR: gio::VolumeMonitor = gio::VolumeMonitor::get();
);

pub fn init() {
    init_device_monitor();
}

fn init_device_monitor() {
    VOLUME_MONITOR.with(|volume_monitor| {
        volume_monitor.connect_mount_added(|_, mount| {
            super::event::mount_added(mount);
        });
    });
}
