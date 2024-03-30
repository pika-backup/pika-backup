pub mod archive_prefix_dialog;
mod check_result_dialog;

pub use check_result_dialog::CheckResultDialog;

use glib::prelude::*;

pub fn init() {
    CheckResultDialog::static_type();
}
