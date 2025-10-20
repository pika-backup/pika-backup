/// System Bus
pub async fn system_connection() -> Result<zbus::Connection, zbus::Error> {
    static CONNECTION: smol::lock::Mutex<Option<zbus::Connection>> = smol::lock::Mutex::new(None);

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
    static PROXY: smol::lock::Mutex<Option<zbus::fdo::DBusProxy<'static>>> =
        smol::lock::Mutex::new(None);

    let mut proxy = PROXY.lock().await;

    if let Some(proxy) = &*proxy {
        Ok(proxy.clone())
    } else {
        let new_proxy = zbus::fdo::DBusProxy::new(session_connection).await?;
        *proxy = Some(new_proxy.clone());
        Ok(new_proxy.clone())
    }
}
