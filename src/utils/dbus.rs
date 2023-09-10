use once_cell::sync::OnceCell;

/// Session Bus
pub async fn session() -> Result<zbus::Connection, zbus::Error> {
    static CONNECTION: OnceCell<zbus::Connection> = OnceCell::new();

    if let Some(connection) = CONNECTION.get() {
        Ok(connection.clone())
    } else {
        let connection = zbus::Connection::session().await?;
        Ok(CONNECTION.get_or_init(move || connection).clone())
    }
}

/// System Bus
pub async fn system() -> Result<zbus::Connection, zbus::Error> {
    static CONNECTION: OnceCell<zbus::Connection> = OnceCell::new();

    if let Some(connection) = CONNECTION.get() {
        Ok(connection.clone())
    } else {
        let connection = zbus::Connection::system().await?;
        Ok(CONNECTION.get_or_init(move || connection).clone())
    }
}

/// FDO proxy
pub async fn fdo_proxy() -> Result<zbus::fdo::DBusProxy<'static>, zbus::Error> {
    static PROXY: OnceCell<zbus::fdo::DBusProxy<'static>> = OnceCell::new();

    if let Some(proxy) = PROXY.get() {
        Ok(proxy.clone())
    } else {
        let proxy = zbus::fdo::DBusProxy::new(&crate::utils::dbus::system().await?).await?;
        Ok(PROXY.get_or_init(move || proxy).clone())
    }
}
