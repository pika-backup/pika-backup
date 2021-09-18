use crate::action;
use crate::config;
use crate::daemon;
use crate::daemon::prelude::*;
use daemon::error::Result;

use gio::prelude::*;

thread_local! {
    static CONFIG_MONITOR: gio::FileMonitor = init_config_monitor();
    static HISTORY_MONITOR: gio::FileMonitor = init_history_monitor();
}

pub fn init() {
    gio_app().connect_startup(on_startup);
}

fn on_startup(_app: &gio::Application) {
    gio_app().hold();
    load_config().handle("Initial config load failed");
    let action = crate::action::backup_start();
    action.connect_activate(daemon::utils::redirect_action(vec![
        action::backup_show(),
        action::backup_start(),
    ]));
    gio_app().add_action(&action);

    CONFIG_MONITOR.with(|_| {});
    HISTORY_MONITOR.with(|_| {});

    daemon::connect::init::init();
    daemon::schedule::init::init();
}

fn init_config_monitor() -> gio::FileMonitor {
    let file =
        gio::File::for_path(&config::Backups::default_path().expect("TODO: we need a config"));
    let monitor = file
        .monitor_file(gio::FileMonitorFlags::NONE, None::<&gio::Cancellable>)
        .expect("TODO: we need a config");
    monitor.connect_changed(on_config_change);

    debug!("Config file monitor connected");
    monitor
}

fn init_history_monitor() -> gio::FileMonitor {
    let file =
        gio::File::for_path(&config::Histories::default_path().expect("TODO: we need a history"));
    let monitor = file
        .monitor_file(gio::FileMonitorFlags::NONE, None::<&gio::Cancellable>)
        .expect("TODO: we need a history");
    monitor.connect_changed(on_config_change);

    debug!("History file monitor connected");
    monitor
}

fn on_config_change(
    _monitor: &gio::FileMonitor,
    _file: &gio::File,
    _other_file: Option<&gio::File>,
    event: gio::FileMonitorEvent,
) {
    debug!("Config file event: {}", event);
    if event == gio::FileMonitorEvent::ChangesDoneHint {
        info!("Reloading config");
        load_config().handle("Reloading config failed");
    }
}

fn load_config() -> Result<()> {
    let conf = config::Backups::from_default_path()?;
    BACKUP_CONFIG.update(move |s| *s = conf.clone());

    let history = config::Histories::from_default_path()?;
    BACKUP_HISTORY.update(|s| *s = history.clone());

    Ok(())
}
