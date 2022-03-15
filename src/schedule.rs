pub mod requirements;

use std::time::Duration;

/// Time in seconds after which the computer is consider "in use"
pub static USED_THRESHOLD: Duration = Duration::from_secs(10 * 60);
