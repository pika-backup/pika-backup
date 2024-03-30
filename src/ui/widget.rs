mod app_window;
mod archives_page;
mod backup_page;
mod detail_dialog;
mod detail_page;
pub mod dialog;
mod encryption_preferences_group;
mod location_tag;
mod overview_page;
mod schedule_page;
mod status_icon;
mod status_row;
mod wrap_box;

pub use app_window::AppWindow;
pub use archives_page::ArchivesPage;
pub use backup_page::BackupPage;
pub use detail_dialog::DetailDialog;
pub use detail_page::DetailPage;
pub use encryption_preferences_group::EncryptionPreferencesGroup;
pub use location_tag::LocationTag;
pub use overview_page::OverviewPage;
pub use schedule_page::{status::Status, SchedulePage};
pub use status_icon::StatusIcon;
pub use status_row::StatusRow;
pub use wrap_box::WrapBox;

use crate::ui;
use glib::prelude::*;

pub fn init() {
    schedule_page::frequency::FrequencyObject::static_type();
    schedule_page::prune_preset::PrunePresetObject::static_type();
    schedule_page::weekday::WeekdayObject::static_type();
    ui::dialog_setup::folder_button::FolderButton::static_type();
    ui::dialog_setup::add_task::AddConfigTask::static_type();
    dialog::CheckResultDialog::static_type();
    ArchivesPage::static_type();
    BackupPage::static_type();
    DetailDialog::static_type();
    DetailPage::static_type();
    EncryptionPreferencesGroup::static_type();
    OverviewPage::static_type();
    SchedulePage::static_type();
    StatusIcon::static_type();
    StatusRow::static_type();
    WrapBox::static_type();
    AppWindow::static_type();
}
