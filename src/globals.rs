use once_cell::sync::Lazy;

pub static ZBUS_SESSION: Lazy<zbus::Connection> = Lazy::new(|| {
    async_std::task::block_on(async {
        zbus::Connection::session()
            .await
            .expect("Failed to create ZBus session connection.")
    })
});
