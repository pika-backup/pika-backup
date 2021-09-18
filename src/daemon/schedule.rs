/*!
# Scheduled backups
*/

pub mod init;
pub mod requirements;
pub mod status;

/// Time in seconds after which the computer is consider "in use"
static MIN_USAGE: u32 = 10 * 60;
