use crate::daemon::prelude::*;

use crate::config;

fn write_result() -> crate::daemon::error::Result<()> {
    let schedule_status: &config::ScheduleStatus = &SCHEDULE_STATUS.load();
    let tmpfile = tempfile::NamedTempFile::new_in(glib::user_config_dir())?;
    serde_json::ser::to_writer_pretty(&tmpfile, schedule_status)?;
    tmpfile.persist(&config::ScheduleStatus::default_path()?)?;

    Ok(())
}

pub fn write() {
    write_result().handle("Could not write schedule status.");
}

fn load_result() -> crate::daemon::error::Result<()> {
    let schedule_status = config::ScheduleStatus::load()?;
    SCHEDULE_STATUS.update(|s| *s = schedule_status.clone());

    Ok(())
}

pub fn load() {
    load_result().expect("Could not load schedule status.");
}
