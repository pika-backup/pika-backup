mod archives_page;
mod backup_page;
mod encryption_preferences_group;
mod location_tag;
mod status_icon;
mod status_row;
mod wrap_box;

pub use backup_page::BackupPage;
pub use encryption_preferences_group::EncryptionPreferencesGroup;
pub use location_tag::LocationTag;
pub use status_icon::StatusIcon;
pub use status_row::StatusRow;
pub use wrap_box::WrapBox;

use crate::ui;
use glib::prelude::*;

pub fn init() {
    ui::page_schedule::frequency::FrequencyObject::static_type();
    ui::page_schedule::prune_preset::PrunePresetObject::static_type();
    ui::page_schedule::weekday::WeekdayObject::static_type();
    ui::dialog_setup::folder_button::FolderButton::static_type();
    ui::dialog_setup::add_task::AddConfigTask::static_type();
    ui::dialog_check_result::DialogCheckResult::static_type();
    BackupPage::static_type();
    EncryptionPreferencesGroup::static_type();
    StatusIcon::static_type();
    StatusRow::static_type();
    WrapBox::static_type();
}
