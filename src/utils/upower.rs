use once_cell::sync::OnceCell;
use zbus::Result;

#[zbus::dbus_proxy(
    default_service = "org.freedesktop.UPower",
    interface = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower",
    assume_defaults = false
)]
trait UPower {
    #[dbus_proxy(property)]
    fn on_battery(&self) -> Result<bool>;
}

pub struct UPower;

impl UPower {
    async fn proxy() -> Result<UPowerProxy<'static>> {
        static PROXY: once_cell::sync::OnceCell<crate::utils::upower::UPowerProxy<'static>> =
            OnceCell::new();

        if let Some(proxy) = PROXY.get() {
            Ok(proxy.clone())
        } else {
            let proxy = UPowerProxy::new(&crate::utils::dbus::system().await?).await?;
            Ok(PROXY.get_or_init(move || proxy).clone())
        }
    }

    pub async fn on_battery() -> Option<bool> {
        if let Ok(proxy) = Self::proxy().await {
            let result = proxy.on_battery().await;
            if let Err(err) = &result {
                warn!("UPower OnBattery() failed: {}", err);
            }

            result.ok()
        } else {
            None
        }
    }

    pub async fn receive_on_battery_changed() -> Option<zbus::PropertyStream<'static, bool>> {
        if let Ok(proxy) = Self::proxy().await {
            let result = proxy.receive_on_battery_changed().await;

            Some(result)
        } else {
            None
        }
    }
}
