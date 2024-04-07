pub mod requirements;

pub use requirements::DueCause;

use std::time::Duration;

/// Time in seconds after which the device is considered "in use" for
/// long enough to start a scheduled backup
pub static USED_THRESHOLD: Duration = Duration::from_secs(10 * 60);
