use crate::daemon::prelude::*;

use crate::config;
use crate::config::ArcSwapWriteable;
use crate::config::Loadable;

fn try_write() -> crate::daemon::error::Result<()> {
    SCHEDULE_STATUS.write_file()?;

    Ok(())
}

pub fn write() {
    try_write().handle("Could not Write Schedule Status");
}

fn try_load() -> crate::daemon::error::Result<()> {
    let schedule_status = config::ScheduleStatus::from_file()?;
    SCHEDULE_STATUS.update(|s| *s = schedule_status.clone());

    Ok(())
}

pub fn load() {
    try_load().handle("Could not Load Schedule Status");
}
