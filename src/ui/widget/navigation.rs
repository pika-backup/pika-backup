mod dialog_page;
mod spinner_page;

pub use dialog_page::{DialogPage, DialogPagePropertiesExt, PkDialogPageImpl};
pub use spinner_page::{PkSpinnerPageImpl, SpinnerPage, SpinnerPagePropertiesExt};

use glib::types::StaticTypeExt;

pub fn init() {
    DialogPage::ensure_type();
    SpinnerPage::ensure_type();
}
