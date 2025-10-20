use std::sync::Arc;

use common::config;
use common::config::{ArcSwapWriteable, Loadable};

use crate::prelude::*;

async fn try_write() -> crate::error::Result<()> {
    SCHEDULE_STATUS.write_file().await?;

    Ok(())
}

pub async fn write() {
    try_write().await.handle("Could not Write Schedule Status");
}

fn try_load() -> crate::error::Result<()> {
    SCHEDULE_STATUS.swap(Arc::new(config::Writeable::from_file()?));

    Ok(())
}

pub fn load() {
    try_load().handle("Could not Load Schedule Status");
}
