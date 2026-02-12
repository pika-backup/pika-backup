pub mod communication;
pub mod error;
pub mod functions;
pub mod invert_command;
pub mod json;
pub mod log_json;
pub mod prelude;
mod process;
pub mod scripts;
pub mod size_estimate;
pub mod status;
pub mod task;
mod utils;

pub use communication::*;
pub use error::{Abort, Error, Failure, Outcome, Result};
pub use functions::*;
pub use json::*;
pub use status::*;
pub use task::Task;

pub static DELAY_RECONNECT: std::time::Duration = std::time::Duration::from_secs(60);
pub static MAX_RECONNECT: u16 = 30;
pub static LOCK_WAIT_RECONNECT: std::time::Duration = std::time::Duration::from_secs(60 * 7);

pub static MESSAGE_POLL_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(100);

/// Duration without new borg log output after which the status is set to
/// [`status::Run::Stalled`]
///
/// TODO: Increase before release
pub static STALL_THRESHOLD: std::time::Duration = std::time::Duration::from_secs(60 * 2);

/// require borg 1.4.0 because of new messages
pub const MIN_VERSION: [u32; 3] = [1, 4, 0];
pub const MAX_VERSION: [u32; 2] = [1, 4];
