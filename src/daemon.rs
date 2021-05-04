#[allow(clippy::from_over_into)]
mod globals;
mod prelude;

use gio::prelude::*;

use crate::action;
use crate::config;
use prelude::*;

use std::cell::RefCell;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

trait Logable {
    fn handle(&self, msg: &str);
}

impl Logable for Result<()> {
    fn handle(&self, msg: &str) {
        if let Err(err) = self {
            error!("Error: {}: {}", msg, err);
        }
    }
}

pub fn main() {
    gio_app().connect_startup(on_startup);
    gio_app().run();
}

thread_local!(
    static SERVICE: Service = Service {
        volume_monitor: gio::VolumeMonitor::get(),
        config_monitor: Default::default(),
    }
);

fn load_config() -> Result<()> {
    let conf = config::Backups::from_default_path()?;
    BACKUP_CONFIG.update(move |s| *s = conf.clone());
    Ok(())
}

struct Service {
    volume_monitor: gio::VolumeMonitor,
    config_monitor: RefCell<Option<gio::FileMonitor>>,
}

fn forward_action(action: &gio::SimpleAction, target_value: Option<&glib::Variant>) {
    debug!(
        "Forwarding action: {:?}",
        gio::Action::print_detailed_name(&action.name(), target_value)
    );
    let dbus_connection = gio_app().dbus_connection().unwrap();
    let group = gio::DBusActionGroup::get(
        &dbus_connection,
        Some(&crate::app_id()),
        &format!("/{}", crate::app_id().replace(".", "/")),
    );
    group.activate_action(&action.name(), target_value);
}

fn redirect_action(
    new_actions: Vec<gio::SimpleAction>,
) -> impl Fn(&gio::SimpleAction, Option<&glib::Variant>) {
    move |action: &gio::SimpleAction, target_value: Option<&glib::Variant>| {
        debug!(
            "Redirecting action: {:?}",
            gio::Action::print_detailed_name(&action.name(), target_value)
        );
        for action in &new_actions {
            forward_action(action, target_value)
        }
    }
}

fn on_startup(_app: &gio::Application) {
    gio_app().hold();
    load_config().handle("Initial config load failed");
    let action = crate::action::backup_start();
    action.connect_activate(redirect_action(vec![
        action::backup_show(),
        action::backup_start(),
    ]));
    gio_app().add_action(&action);

    init_config_monitor().handle("Failed to initialize config file monitor");
    init_device_monitor();
}

fn init_config_monitor() -> Result<()> {
    let file = gio::File::new_for_path(&config::Backups::default_path()?);
    let monitor = file.monitor_file(gio::FileMonitorFlags::NONE, None::<&gio::Cancellable>)?;
    monitor.connect_changed(on_config_change);
    SERVICE.with(|service| {
        *service.config_monitor.borrow_mut() = Some(monitor);
    });
    debug!("Config file monitor connected");
    Ok(())
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

fn init_device_monitor() {
    SERVICE.with(|service| {
        service.volume_monitor.connect_mount_added(|_, mount| {
            let backups = &BACKUP_CONFIG.load();
            let uuid = crate::utils::mount_uuid(mount);
            debug!("Log: Connected {:?}", uuid);
            if let Some(uuid) = uuid {
                let backup = backups.iter().find(|b| {
                    debug!("Log: Checking {:?}", &b);
                    if let config::Backup {
                        repo: config::Repository::Local(local),
                        ..
                    } = b
                    {
                        local.volume_uuid.as_ref() == Some(&uuid)
                    } else {
                        false
                    }
                });

                if let Some(config::Backup {
                    id,
                    repo: config::Repository::Local(local),
                    ..
                }) = backup
                {
                    let notification = gio::Notification::new("Backup Medium Connected");
                    notification.set_body(Some(
                        format!(
                            "{} on Disk '{}'",
                            local.mount_name.as_ref().unwrap(),
                            local.drive_name.as_ref().unwrap()
                        )
                        .as_str(),
                    ));

                    notification.add_button_with_target_value(
                        "Run Backup",
                        &format!("app.{}", crate::action::backup_start().name()),
                        Some(&id.to_string().to_variant()),
                    );
                    gio_app().send_notification(Some(uuid.as_str()), &notification);
                    debug!("Log: Notification send");
                }
            }
        });
    });
}
