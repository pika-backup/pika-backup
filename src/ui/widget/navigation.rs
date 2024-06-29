mod dialog_page;

pub use dialog_page::{DialogPage, DialogPagePropertiesExt, PkDialogPageImpl};

use glib::types::StaticTypeExt;

pub fn init() {
    DialogPage::ensure_type();
}
