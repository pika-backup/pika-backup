use super::prelude::*;
use arc_swap::ArcSwap;

static LAST_MESSAGE: once_cell::sync::Lazy<ArcSwap<Option<String>>> =
    once_cell::sync::Lazy::new(|| ArcSwap::new(Default::default()));

async fn proxy() -> Option<Arc<ashpd::desktop::background::BackgroundProxy<'static>>> {
    static PROXY: once_cell::sync::OnceCell<
        Arc<ashpd::desktop::background::BackgroundProxy<'static>>,
    > = once_cell::sync::OnceCell::new();

    if let Some(proxy) = PROXY.get() {
        Some(proxy.clone())
    } else {
        ashpd::desktop::background::BackgroundProxy::new()
            .await
            .ok()
            .map(|proxy| PROXY.get_or_init(move || Arc::new(proxy)).clone())
    }
}

pub async fn background_activity_update() {
    let message = BORG_OPERATION.with(|operations| operations.load().summarize_operations());
    set_status_message(&message.unwrap_or(gettext("Idle"))).await;
}

pub async fn set_status_message(message: &str) {
    let ellipsized_message = crate::ui::utils::ellipsize_end(message, 96);
    let new_message = Arc::new(Some(ellipsized_message.clone()));
    let last_message = LAST_MESSAGE.swap(new_message.clone());

    if *last_message != *new_message {
        debug!("New background status: {new_message:?}");

        if !*crate::globals::APP_IS_SANDBOXED {
            return;
        }

        if let Some(proxy) = proxy().await {
            if let Err(err) = proxy.set_status(&ellipsized_message).await {
                error!("Error setting background status: {err:?}");
            }
        }
    }
}
