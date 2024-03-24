use super::prelude::*;
use arc_swap::ArcSwap;

static LAST_MESSAGE: once_cell::sync::Lazy<ArcSwap<Option<String>>> =
    once_cell::sync::Lazy::new(|| ArcSwap::new(Default::default()));

async fn proxy() -> Option<Arc<ashpd::desktop::background::BackgroundProxy<'static>>> {
    static PROXY: async_lock::Mutex<
        Option<Arc<ashpd::desktop::background::BackgroundProxy<'static>>>,
    > = async_lock::Mutex::new(None);

    let mut proxy = PROXY.lock().await;

    if let Some(proxy) = &*proxy {
        Some(proxy.clone())
    } else {
        let new_proxy = Arc::new(
            ashpd::desktop::background::BackgroundProxy::new()
                .await
                .ok()?,
        );
        *proxy = Some(new_proxy.clone());
        Some(new_proxy.clone())
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
                debug!("Error setting background status: {err:?}");
            }
        }
    }
}
