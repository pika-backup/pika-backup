use crate::daemon::prelude::*;

use crate::config;
use crate::config::Loadable;

fn write_result() -> crate::daemon::error::Result<()> {
    SCHEDULE_STATUS.load().write_file()?;

    Ok(())
}

pub fn write() {
    write_result().handle("Could not write schedule status.");
}

fn load_result() -> crate::daemon::error::Result<()> {
    let schedule_status = config::ScheduleStatus::from_file()?;
    SCHEDULE_STATUS.update(|s| *s = schedule_status.clone());

    Ok(())
}

pub fn load() {
    load_result().expect("Could not load schedule status.");
}
