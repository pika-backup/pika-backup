mod archive_prefix_dialog;
mod check_result_dialog;
pub mod prune_review_dialog;

pub use archive_prefix_dialog::ArchivePrefixDialog;
pub use check_result_dialog::CheckResultDialog;

use glib::prelude::*;

pub fn init() {
    CheckResultDialog::static_type();
}
