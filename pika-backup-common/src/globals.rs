use std::sync::LazyLock;

const CLOCK_INTERFACE: &str = "org.gnome.desktop.interface";
const CLOCK_KEY: &str = "clock-format";

pub static APP_IS_SANDBOXED: LazyLock<bool> =
    LazyLock::new(|| smol::block_on(ashpd::is_sandboxed()));

pub static CLOCK_IS_24H: LazyLock<bool> = LazyLock::new(|| {
    smol::block_on(async {
        let proxy = ashpd::desktop::settings::Settings::new().await?;
        proxy.read::<String>(CLOCK_INTERFACE, CLOCK_KEY).await
    })
    .map(|s| s == "24h")
    .inspect_err(|e| tracing::warn!("Retrieving '{}' setting failed: {}", CLOCK_KEY, e))
    .unwrap_or_default()
});

pub static MEMORY_PASSWORD_STORE: LazyLock<
    std::sync::Arc<crate::utils::password::MemoryPasswordStore>,
> = LazyLock::new(Default::default);
