use super::prelude::*;

static BACKGROUND_PROXY: once_cell::sync::OnceCell<
    Arc<ashpd::desktop::background::BackgroundProxy<'static>>,
> = once_cell::sync::OnceCell::new();

async fn proxy() -> Option<Arc<ashpd::desktop::background::BackgroundProxy<'static>>> {
    match BACKGROUND_PROXY.get() {
        Some(proxy) => Some(proxy.clone()),
        None => ashpd::desktop::background::BackgroundProxy::new()
            .await
            .ok()
            .map(|proxy| {
                BACKGROUND_PROXY
                    .get_or_init(move || Arc::new(proxy))
                    .clone()
            }),
    }
}

pub async fn background_activity_update() {
    if let Some(proxy) = proxy().await {
        let message = BORG_OPERATION.with(|operations| operations.load().summarize_operations());
        if let Err(err) = proxy.set_status(&message.unwrap_or(gettext("Idle"))).await {
            error!("Error setting background status: {err:?}");
        }
    }
}

pub async fn set_custom(message: &str) {
    if let Some(proxy) = proxy().await {
        if let Err(err) = proxy.set_status(message).await {
            error!("Error setting background status: {err:?}");
        }
    }
}
