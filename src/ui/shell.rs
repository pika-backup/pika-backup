use super::prelude::*;
use arc_swap::ArcSwap;

static BACKGROUND_PROXY: once_cell::sync::OnceCell<
    Arc<ashpd::desktop::background::BackgroundProxy<'static>>,
> = once_cell::sync::OnceCell::new();
static LAST_MESSAGE: once_cell::sync::Lazy<ArcSwap<Option<String>>> =
    once_cell::sync::Lazy::new(|| ArcSwap::new(Default::default()));

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
    let message = BORG_OPERATION.with(|operations| operations.load().summarize_operations());
    set_status_message(&message.unwrap_or(gettext("Idle"))).await;
}

pub async fn set_status_message(message: &str) {
    let new_message = Arc::new(Some(message.to_string()));
    let last_message = LAST_MESSAGE.swap(new_message.clone());

    if *last_message != *new_message {
        if let Some(proxy) = proxy().await {
            if let Err(err) = proxy.set_status(message).await {
                error!("Error setting background status: {err:?}");
            }
        }
    }
}
