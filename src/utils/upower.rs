use zbus::Result;

#[zbus::proxy(
    default_service = "org.freedesktop.UPower",
    interface = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower",
    assume_defaults = false
)]
trait UPower {
    #[zbus(property)]
    fn on_battery(&self) -> Result<bool>;
}

pub struct UPower;

impl UPower {
    async fn proxy() -> Result<UPowerProxy<'static>> {
        static PROXY: smol::lock::Mutex<Option<crate::utils::upower::UPowerProxy<'static>>> =
            smol::lock::Mutex::new(None);

        let mut proxy = PROXY.lock().await;

        if let Some(proxy) = &*proxy {
            Ok(proxy.clone())
        } else {
            let new_proxy =
                UPowerProxy::new(&crate::utils::dbus::system_connection().await?).await?;
            *proxy = Some(new_proxy.clone());
            Ok(new_proxy.clone())
        }
    }

    pub async fn on_battery() -> Option<bool> {
        match Self::proxy().await {
            Ok(proxy) => {
                let result = proxy.on_battery().await;
                if let Err(err) = &result {
                    warn!("UPower OnBattery() failed: {}", err);
                }

                result.ok()
            }
            _ => None,
        }
    }

    pub async fn receive_on_battery_changed() -> Option<zbus::proxy::PropertyStream<'static, bool>>
    {
        match Self::proxy().await {
            Ok(proxy) => {
                let result = proxy.receive_on_battery_changed().await;

                Some(result)
            }
            _ => None,
        }
    }
}
