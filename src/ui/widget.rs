mod app_window;
mod detail;
pub mod dialog;
mod encryption_preferences_group;
mod location_tag;
mod overview;
mod status_icon;
mod status_row;
mod wrap_box;

pub use app_window::AppWindow;
pub use detail::{
    frequency, prune_preset, weekday, ArchivesPage, BackupPage, DetailPage, SchedulePage,
    ScheduleStatus,
};
pub use encryption_preferences_group::EncryptionPreferencesGroup;
pub use location_tag::LocationTag;
pub use overview::OverviewPage;
pub use status_icon::StatusIcon;
pub use status_row::StatusRow;
pub use wrap_box::WrapBox;

pub use dialog::*;

use glib::prelude::*;

pub fn init() {
    frequency::FrequencyObject::static_type();
    prune_preset::PrunePresetObject::static_type();
    weekday::WeekdayObject::static_type();
    ArchivesPage::static_type();
    BackupPage::static_type();
    BackupInfoDialog::static_type();
    DetailPage::static_type();
    EncryptionPreferencesGroup::static_type();
    OverviewPage::static_type();
    SchedulePage::static_type();
    StatusIcon::static_type();
    StatusRow::static_type();
    WrapBox::static_type();
    AppWindow::static_type();

    dialog::init();
}
