/*!
# Scheduled backups
*/

pub mod init;
pub mod requirements;
pub mod status;

pub use super::error::Logable;

use std::time::Duration;

/// Time in seconds after which the computer is consider "in use"
pub static USED_THRESHOLD: Duration = Duration::from_secs(10 * 60);
// TODO: Adjust before release
pub static SCHEDULE_PROBE_FREQUENCY: Duration = Duration::from_secs(10);
