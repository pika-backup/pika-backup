use crate::prelude::*;

use once_cell::sync::Lazy;
use zbus::Result;

#[zbus::dbus_proxy(
    default_service = "org.freedesktop.UPower",
    interface = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower"
)]
trait UPower {
    #[dbus_proxy(property)]
    fn on_battery(&self) -> Result<bool>;
}

static UPOWER_PROXY: Lazy<Option<crate::utils::upower::UPowerProxy<'static>>> = Lazy::new(|| {
    async_std::task::block_on(async {
        let proxy = crate::utils::upower::UPower::proxy().await;
        if let Err(err) = &proxy {
            warn!("Failed to get UPower information: {}", err);
        }

        proxy.ok()
    })
});

pub struct UPower;

impl UPower {
    async fn proxy() -> Result<UPowerProxy<'static>> {
        UPowerProxy::new(&ZBUS_SYSTEM).await
    }

    pub async fn on_battery() -> Option<bool> {
        if let Some(proxy) = &*UPOWER_PROXY {
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
        if let Some(proxy) = &*UPOWER_PROXY {
            let result = proxy.receive_on_battery_changed().await;

            Some(result)
        } else {
            None
        }
    }
}
