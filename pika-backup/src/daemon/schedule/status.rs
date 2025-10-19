use crate::config;
use crate::config::{ArcSwapWriteable, Loadable};
use crate::daemon::prelude::*;

async fn try_write() -> crate::daemon::error::Result<()> {
    SCHEDULE_STATUS.write_file().await?;

    Ok(())
}

pub async fn write() {
    try_write().await.handle("Could not Write Schedule Status");
}

fn try_load() -> crate::daemon::error::Result<()> {
    SCHEDULE_STATUS.swap(Arc::new(config::Writeable::from_file()?));

    Ok(())
}

pub fn load() {
    try_load().handle("Could not Load Schedule Status");
}
