use gio::ActionGroup;
use gio::DBusConnection;
use gio::RemoteActionGroup;
use glib::glib_wrapper;
use glib::translate::*;
use std::fmt;

glib_wrapper! {
    pub struct DBusActionGroup(Object<gio_sys::GDBusActionGroup, DBusActionGroupClass>) @implements ActionGroup, RemoteActionGroup;

    match fn {
        get_type => || gio_sys::g_dbus_action_group_get_type(),
    }
}

impl DBusActionGroup {
    pub fn get(
        connection: &DBusConnection,
        bus_name: Option<&str>,
        object_path: &str,
    ) -> Option<DBusActionGroup> {
        unsafe {
            from_glib_full(gio_sys::g_dbus_action_group_get(
                connection.to_glib_none().0,
                bus_name.to_glib_none().0,
                object_path.to_glib_none().0,
            ))
        }
    }
}

impl fmt::Display for DBusActionGroup {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "DBusActionGroup")
    }
}
