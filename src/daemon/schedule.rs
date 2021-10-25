/*!
# Scheduled backups
*/

pub mod init;
pub mod requirements;
pub mod status;

// Time in seconds after which the computer is consider "in use"
pub static USED_THRESHOLD: std::time::Duration = std::time::Duration::from_secs(10 * 60);
