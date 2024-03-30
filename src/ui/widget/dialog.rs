mod archive_prefix_dialog;
mod check_result_dialog;
pub mod prune_dialog;
mod prune_review_dialog;

pub use archive_prefix_dialog::ArchivePrefixDialog;
pub use check_result_dialog::CheckResultDialog;
pub use prune_review_dialog::PruneReviewDialog;

use glib::prelude::*;

pub fn init() {
    ArchivePrefixDialog::static_type();
    CheckResultDialog::static_type();
    PruneReviewDialog::static_type();
}
