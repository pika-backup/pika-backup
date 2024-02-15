/// System Bus
pub async fn system_connection() -> Result<zbus::Connection, zbus::Error> {
    static CONNECTION: async_lock::Mutex<Option<zbus::Connection>> = async_lock::Mutex::new(None);

    let mut connection = CONNECTION.lock().await;

    if let Some(connection) = &*connection {
        Ok(connection.clone())
    } else {
        let new_connection = zbus::Connection::system().await?;
        *connection = Some(new_connection.clone());
        Ok(new_connection)
    }
}

/// FDO proxy
pub async fn fdo_proxy(
    session_connection: &zbus::Connection,
) -> Result<zbus::fdo::DBusProxy<'static>, zbus::Error> {
    static PROXY: async_lock::Mutex<Option<zbus::fdo::DBusProxy<'static>>> =
        async_lock::Mutex::new(None);

    let mut proxy = PROXY.lock().await;

    if let Some(proxy) = &*proxy {
        Ok(proxy.clone())
    } else {
        let new_proxy = zbus::fdo::DBusProxy::new(session_connection).await?;
        *proxy = Some(new_proxy.clone());
        Ok(new_proxy.clone())
    }
}
