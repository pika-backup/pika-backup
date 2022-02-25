use crate::daemon::prelude::*;
use gio::prelude::*;

pub fn forward_action(action: &gio::SimpleAction, target_value: Option<&glib::Variant>) {
    debug!(
        "Forwarding action: {:?}",
        gio::Action::print_detailed_name(&action.name(), target_value)
    );
    let dbus_connection = gio_app().dbus_connection().unwrap();
    let group = gio::DBusActionGroup::get(
        &dbus_connection,
        Some(&crate::app_id()),
        &format!("/{}", crate::app_id().replace('.', "/")),
    );
    group.activate_action(&action.name(), target_value);
}

pub fn redirect_action(
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
