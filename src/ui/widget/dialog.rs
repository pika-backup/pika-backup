mod archive_prefix_dialog;
mod check_dialog;
mod check_result_dialog;
pub mod delete_archive_dialog;
mod exclude_dialog;
mod prune_dialog;
mod prune_review_dialog;

pub use archive_prefix_dialog::ArchivePrefixDialog;
pub use check_dialog::CheckDialog;
pub use check_result_dialog::CheckResultDialog;
pub use exclude_dialog::ExcludeDialog;
pub use prune_dialog::PruneDialog;
pub use prune_review_dialog::PruneReviewDialog;

use glib::prelude::*;

pub fn init() {
    ArchivePrefixDialog::static_type();
    CheckResultDialog::static_type();
    ExcludeDialog::static_type();
    PruneDialog::static_type();
    PruneReviewDialog::static_type();
}
