/*!
# Scheduled backups
*/

pub mod init;
pub mod status;

pub use super::error::Logable;

use std::time::Duration;

// TODO: Adjust before release
pub static PROBE_FREQUENCY: Duration = Duration::from_secs(10);

/// Remind daily about backups currently not happening due to unmet criteria
pub static REMIND_UNMET_CRITERIA: Duration = Duration::from_secs(24 * 60 * 60);
