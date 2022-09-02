#[derive(Clone)]
pub struct AppWindow {
    builder: gtk::Builder,
}

#[derive(Clone)]
pub struct AppWindowWeak {
    builder: glib::WeakRef<gtk::Builder>,
}

impl glib::clone::Downgrade for AppWindow {
    type Weak = AppWindowWeak;

    fn downgrade(&self) -> Self::Weak {
        Self::Weak {
            builder: self.builder.downgrade(),
        }
    }
}

impl glib::clone::Upgrade for AppWindowWeak {
    type Strong = AppWindow;

    fn upgrade(&self) -> Option<Self::Strong> {
        Some(Self::Strong {
            builder: self.builder.upgrade()?,
        })
    }
}

impl AppWindow {
    pub fn new() -> Self {
        Self {
            builder: gtk::Builder::from_string(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/app_window.ui"
            ))),
        }
    }

    fn get<T: glib::IsA<glib::object::Object>>(&self, id: &str) -> T {
        gtk::Builder::object(&self.builder, id).unwrap_or_else(|| {
            panic!(
                "Object with id '{}' not found in 'src/ui/app_window.ui'",
                id
            )
        })
    }

    pub fn add_backup(&self) -> gtk::Button {
        self.get("add_backup")
    }

    pub fn add_backup_empty(&self) -> gtk::Button {
        self.get("add_backup_empty")
    }

    pub fn add_exclude(&self) -> gtk::Button {
        self.get("add_exclude")
    }

    pub fn add_include(&self) -> gtk::Button {
        self.get("add_include")
    }

    pub fn archive_list(&self) -> gtk::ListBox {
        self.get("archive_list")
    }

    pub fn archive_list_placeholder(&self) -> gtk::ListBox {
        self.get("archive_list_placeholder")
    }

    pub fn archives_cleanup(&self) -> adw::ActionRow {
        self.get("archives_cleanup")
    }

    pub fn archives_eject_button(&self) -> gtk::Button {
        self.get("archives_eject_button")
    }

    pub fn archives_fs_usage(&self) -> gtk::LevelBar {
        self.get("archives_fs_usage")
    }

    pub fn archives_location_icon(&self) -> gtk::Image {
        self.get("archives_location_icon")
    }

    pub fn archives_location_subtitle(&self) -> gtk::Label {
        self.get("archives_location_subtitle")
    }

    pub fn archives_location_suffix_subtitle(&self) -> gtk::Label {
        self.get("archives_location_suffix_subtitle")
    }

    pub fn archives_location_suffix_title(&self) -> gtk::Label {
        self.get("archives_location_suffix_title")
    }

    pub fn archives_location_title(&self) -> gtk::Label {
        self.get("archives_location_title")
    }

    pub fn archives_prefix(&self) -> gtk::Label {
        self.get("archives_prefix")
    }

    pub fn archives_prefix_edit(&self) -> gtk::Button {
        self.get("archives_prefix_edit")
    }

    pub fn archives_reloading_spinner(&self) -> gtk::Spinner {
        self.get("archives_reloading_spinner")
    }

    pub fn archives_reloading_stack(&self) -> gtk::Stack {
        self.get("archives_reloading_stack")
    }

    pub fn archives_stack(&self) -> gtk::Stack {
        self.get("archives_stack")
    }

    pub fn back_button(&self) -> gtk::Button {
        self.get("back_button")
    }

    pub fn backup_exclude(&self) -> gtk::ListBox {
        self.get("backup_exclude")
    }

    pub fn backup_run(&self) -> gtk::Button {
        self.get("backup_run")
    }

    pub fn detail_current_path(&self) -> gtk::Label {
        self.get("detail_current_path")
    }

    pub fn detail_deduplicated_size(&self) -> gtk::Label {
        self.get("detail_deduplicated_size")
    }

    pub fn detail_exclude_placeholder(&self) -> gtk::ListBox {
        self.get("detail_exclude_placeholder")
    }

    pub fn detail_exclude_stack(&self) -> gtk::Stack {
        self.get("detail_exclude_stack")
    }

    pub fn detail_hint_icon(&self) -> gtk::Image {
        self.get("detail_hint_icon")
    }

    pub fn detail_info_error(&self) -> gtk::Label {
        self.get("detail_info_error")
    }

    pub fn detail_info_progress(&self) -> gtk::ProgressBar {
        self.get("detail_info_progress")
    }

    pub fn detail_info_status(&self) -> gtk::Label {
        self.get("detail_info_status")
    }

    pub fn detail_info_substatus(&self) -> gtk::Label {
        self.get("detail_info_substatus")
    }

    pub fn detail_nfiles(&self) -> gtk::Label {
        self.get("detail_nfiles")
    }

    pub fn detail_original_size(&self) -> gtk::Label {
        self.get("detail_original_size")
    }

    pub fn detail_path_row(&self) -> adw::ActionRow {
        self.get("detail_path_row")
    }

    pub fn detail_repo_icon(&self) -> gtk::Image {
        self.get("detail_repo_icon")
    }

    pub fn detail_repo_row(&self) -> adw::ActionRow {
        self.get("detail_repo_row")
    }

    pub fn detail_running_backup_info(&self) -> gtk::Dialog {
        self.get("detail_running_backup_info")
    }

    pub fn detail_stack(&self) -> adw::ViewStack {
        self.get("detail_stack")
    }

    pub fn detail_stats(&self) -> gtk::ListBox {
        self.get("detail_stats")
    }

    pub fn detail_status_row(&self) -> adw::ActionRow {
        self.get("detail_status_row")
    }

    pub fn include(&self) -> gtk::ListBox {
        self.get("include")
    }

    pub fn leaflet(&self) -> adw::Leaflet {
        self.get("leaflet")
    }

    pub fn main_backups(&self) -> gtk::ListBox {
        self.get("main_backups")
    }

    pub fn main_stack(&self) -> adw::ViewStack {
        self.get("main_stack")
    }

    pub fn overview(&self) -> gtk::Box {
        self.get("overview")
    }

    pub fn page_archives(&self) -> adw::PreferencesPage {
        self.get("page_archives")
    }

    pub fn page_backup(&self) -> adw::PreferencesPage {
        self.get("page_backup")
    }

    pub fn page_detail(&self) -> gtk::Box {
        self.get("page_detail")
    }

    pub fn page_overview(&self) -> adw::PreferencesPage {
        self.get("page_overview")
    }

    pub fn page_overview_empty(&self) -> adw::StatusPage {
        self.get("page_overview_empty")
    }

    pub fn page_schedule(&self) -> adw::PreferencesPage {
        self.get("page_schedule")
    }

    pub fn pending_menu(&self) -> gtk::MenuButton {
        self.get("pending_menu")
    }

    pub fn pending_menu_spinner(&self) -> gtk::Spinner {
        self.get("pending_menu_spinner")
    }

    pub fn preferred_day_row(&self) -> adw::ActionRow {
        self.get("preferred_day_row")
    }

    pub fn preferred_time_row(&self) -> adw::ActionRow {
        self.get("preferred_time_row")
    }

    pub fn preferred_weekday_row(&self) -> adw::ComboRow {
        self.get("preferred_weekday_row")
    }

    pub fn primary_menu_button(&self) -> gtk::MenuButton {
        self.get("primary_menu_button")
    }

    pub fn prune_detail(&self) -> adw::ExpanderRow {
        self.get("prune_detail")
    }

    pub fn prune_enabled(&self) -> gtk::Switch {
        self.get("prune_enabled")
    }

    pub fn prune_preset(&self) -> adw::ComboRow {
        self.get("prune_preset")
    }

    pub fn prune_save(&self) -> gtk::Button {
        self.get("prune_save")
    }

    pub fn prune_save_revealer(&self) -> gtk::Revealer {
        self.get("prune_save_revealer")
    }

    pub fn refresh_archives(&self) -> gtk::Button {
        self.get("refresh_archives")
    }

    pub fn schedule_active(&self) -> adw::ExpanderRow {
        self.get("schedule_active")
    }

    pub fn schedule_frequency(&self) -> adw::ComboRow {
        self.get("schedule_frequency")
    }

    pub fn schedule_keep_daily(&self) -> gtk::SpinButton {
        self.get("schedule_keep_daily")
    }

    pub fn schedule_keep_hourly(&self) -> gtk::SpinButton {
        self.get("schedule_keep_hourly")
    }

    pub fn schedule_keep_monthly(&self) -> gtk::SpinButton {
        self.get("schedule_keep_monthly")
    }

    pub fn schedule_keep_weekly(&self) -> gtk::SpinButton {
        self.get("schedule_keep_weekly")
    }

    pub fn schedule_keep_yearly(&self) -> gtk::SpinButton {
        self.get("schedule_keep_yearly")
    }

    pub fn schedule_preferred_day(&self) -> gtk::MenuButton {
        self.get("schedule_preferred_day")
    }

    pub fn schedule_preferred_day_calendar(&self) -> gtk::Calendar {
        self.get("schedule_preferred_day_calendar")
    }

    pub fn schedule_preferred_day_popover(&self) -> gtk::Popover {
        self.get("schedule_preferred_day_popover")
    }

    pub fn schedule_preferred_hour(&self) -> gtk::SpinButton {
        self.get("schedule_preferred_hour")
    }

    pub fn schedule_preferred_minute(&self) -> gtk::SpinButton {
        self.get("schedule_preferred_minute")
    }

    pub fn schedule_preferred_time_button(&self) -> gtk::MenuButton {
        self.get("schedule_preferred_time_button")
    }

    pub fn schedule_preferred_time_popover(&self) -> gtk::Popover {
        self.get("schedule_preferred_time_popover")
    }

    pub fn schedule_status(&self) -> adw::ActionRow {
        self.get("schedule_status")
    }

    pub fn schedule_status_icon(&self) -> crate::ui::export::StatusIcon {
        self.get("schedule_status_icon")
    }

    pub fn schedule_status_list(&self) -> gtk::ListBox {
        self.get("schedule_status_list")
    }

    pub fn status_graphic(&self) -> gtk::Stack {
        self.get("status_graphic")
    }

    pub fn status_icon(&self) -> gtk::Image {
        self.get("status_icon")
    }

    pub fn status_spinner(&self) -> gtk::Spinner {
        self.get("status_spinner")
    }

    pub fn stop_backup_create(&self) -> gtk::Button {
        self.get("stop_backup_create")
    }

    pub fn toast(&self) -> adw::ToastOverlay {
        self.get("toast")
    }

    pub fn view_switcher_title(&self) -> adw::ViewSwitcherTitle {
        self.get("view_switcher_title")
    }

    pub fn window(&self) -> adw::ApplicationWindow {
        self.get("window")
    }
}

#[derive(Clone)]
pub struct DialogAbout {
    builder: gtk::Builder,
}

#[derive(Clone)]
pub struct DialogAboutWeak {
    builder: glib::WeakRef<gtk::Builder>,
}

impl glib::clone::Downgrade for DialogAbout {
    type Weak = DialogAboutWeak;

    fn downgrade(&self) -> Self::Weak {
        Self::Weak {
            builder: self.builder.downgrade(),
        }
    }
}

impl glib::clone::Upgrade for DialogAboutWeak {
    type Strong = DialogAbout;

    fn upgrade(&self) -> Option<Self::Strong> {
        Some(Self::Strong {
            builder: self.builder.upgrade()?,
        })
    }
}

impl DialogAbout {
    pub fn new() -> Self {
        Self {
            builder: gtk::Builder::from_string(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/dialog_about.ui"
            ))),
        }
    }

    fn get<T: glib::IsA<glib::object::Object>>(&self, id: &str) -> T {
        gtk::Builder::object(&self.builder, id).unwrap_or_else(|| {
            panic!(
                "Object with id '{}' not found in 'src/ui/dialog_about.ui'",
                id
            )
        })
    }

    pub fn dialog(&self) -> adw::AboutWindow {
        self.get("dialog")
    }
}

#[derive(Clone)]
pub struct DialogArchivePrefix {
    builder: gtk::Builder,
}

#[derive(Clone)]
pub struct DialogArchivePrefixWeak {
    builder: glib::WeakRef<gtk::Builder>,
}

impl glib::clone::Downgrade for DialogArchivePrefix {
    type Weak = DialogArchivePrefixWeak;

    fn downgrade(&self) -> Self::Weak {
        Self::Weak {
            builder: self.builder.downgrade(),
        }
    }
}

impl glib::clone::Upgrade for DialogArchivePrefixWeak {
    type Strong = DialogArchivePrefix;

    fn upgrade(&self) -> Option<Self::Strong> {
        Some(Self::Strong {
            builder: self.builder.upgrade()?,
        })
    }
}

impl DialogArchivePrefix {
    pub fn new() -> Self {
        Self {
            builder: gtk::Builder::from_string(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/dialog_archive_prefix.ui"
            ))),
        }
    }

    fn get<T: glib::IsA<glib::object::Object>>(&self, id: &str) -> T {
        gtk::Builder::object(&self.builder, id).unwrap_or_else(|| {
            panic!(
                "Object with id '{}' not found in 'src/ui/dialog_archive_prefix.ui'",
                id
            )
        })
    }

    pub fn archive_prefix(&self) -> gtk::Entry {
        self.get("archive_prefix")
    }

    pub fn cancel(&self) -> gtk::Button {
        self.get("cancel")
    }

    pub fn dialog(&self) -> gtk::Dialog {
        self.get("dialog")
    }

    pub fn ok(&self) -> gtk::Button {
        self.get("ok")
    }
}

#[derive(Clone)]
pub struct DialogDeviceMissing {
    builder: gtk::Builder,
}

#[derive(Clone)]
pub struct DialogDeviceMissingWeak {
    builder: glib::WeakRef<gtk::Builder>,
}

impl glib::clone::Downgrade for DialogDeviceMissing {
    type Weak = DialogDeviceMissingWeak;

    fn downgrade(&self) -> Self::Weak {
        Self::Weak {
            builder: self.builder.downgrade(),
        }
    }
}

impl glib::clone::Upgrade for DialogDeviceMissingWeak {
    type Strong = DialogDeviceMissing;

    fn upgrade(&self) -> Option<Self::Strong> {
        Some(Self::Strong {
            builder: self.builder.upgrade()?,
        })
    }
}

impl DialogDeviceMissing {
    pub fn new() -> Self {
        Self {
            builder: gtk::Builder::from_string(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/dialog_device_missing.ui"
            ))),
        }
    }

    fn get<T: glib::IsA<glib::object::Object>>(&self, id: &str) -> T {
        gtk::Builder::object(&self.builder, id).unwrap_or_else(|| {
            panic!(
                "Object with id '{}' not found in 'src/ui/dialog_device_missing.ui'",
                id
            )
        })
    }

    pub fn icon(&self) -> gtk::Box {
        self.get("icon")
    }

    pub fn name(&self) -> gtk::Label {
        self.get("name")
    }

    pub fn window(&self) -> gtk::Dialog {
        self.get("window")
    }
}

#[derive(Clone)]
pub struct DialogEncryptionPassword {
    builder: gtk::Builder,
}

#[derive(Clone)]
pub struct DialogEncryptionPasswordWeak {
    builder: glib::WeakRef<gtk::Builder>,
}

impl glib::clone::Downgrade for DialogEncryptionPassword {
    type Weak = DialogEncryptionPasswordWeak;

    fn downgrade(&self) -> Self::Weak {
        Self::Weak {
            builder: self.builder.downgrade(),
        }
    }
}

impl glib::clone::Upgrade for DialogEncryptionPasswordWeak {
    type Strong = DialogEncryptionPassword;

    fn upgrade(&self) -> Option<Self::Strong> {
        Some(Self::Strong {
            builder: self.builder.upgrade()?,
        })
    }
}

impl DialogEncryptionPassword {
    pub fn new() -> Self {
        Self {
            builder: gtk::Builder::from_string(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/dialog_encryption_password.ui"
            ))),
        }
    }

    fn get<T: glib::IsA<glib::object::Object>>(&self, id: &str) -> T {
        gtk::Builder::object(&self.builder, id).unwrap_or_else(|| {
            panic!(
                "Object with id '{}' not found in 'src/ui/dialog_encryption_password.ui'",
                id
            )
        })
    }

    pub fn cancel(&self) -> gtk::Button {
        self.get("cancel")
    }

    pub fn description(&self) -> gtk::Label {
        self.get("description")
    }

    pub fn dialog(&self) -> gtk::Dialog {
        self.get("dialog")
    }

    pub fn ok(&self) -> gtk::Button {
        self.get("ok")
    }

    pub fn password(&self) -> gtk::Entry {
        self.get("password")
    }
}

#[derive(Clone)]
pub struct DialogExclude {
    builder: gtk::Builder,
}

#[derive(Clone)]
pub struct DialogExcludeWeak {
    builder: glib::WeakRef<gtk::Builder>,
}

impl glib::clone::Downgrade for DialogExclude {
    type Weak = DialogExcludeWeak;

    fn downgrade(&self) -> Self::Weak {
        Self::Weak {
            builder: self.builder.downgrade(),
        }
    }
}

impl glib::clone::Upgrade for DialogExcludeWeak {
    type Strong = DialogExclude;

    fn upgrade(&self) -> Option<Self::Strong> {
        Some(Self::Strong {
            builder: self.builder.upgrade()?,
        })
    }
}

impl DialogExclude {
    pub fn new() -> Self {
        Self {
            builder: gtk::Builder::from_string(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/dialog_exclude.ui"
            ))),
        }
    }

    fn get<T: glib::IsA<glib::object::Object>>(&self, id: &str) -> T {
        gtk::Builder::object(&self.builder, id).unwrap_or_else(|| {
            panic!(
                "Object with id '{}' not found in 'src/ui/dialog_exclude.ui'",
                id
            )
        })
    }

    pub fn dialog(&self) -> adw::Window {
        self.get("dialog")
    }

    pub fn exclude_file(&self) -> adw::ActionRow {
        self.get("exclude_file")
    }

    pub fn exclude_folder(&self) -> adw::ActionRow {
        self.get("exclude_folder")
    }

    pub fn suggestions(&self) -> adw::PreferencesGroup {
        self.get("suggestions")
    }
}

#[derive(Clone)]
pub struct DialogPrune {
    builder: gtk::Builder,
}

#[derive(Clone)]
pub struct DialogPruneWeak {
    builder: glib::WeakRef<gtk::Builder>,
}

impl glib::clone::Downgrade for DialogPrune {
    type Weak = DialogPruneWeak;

    fn downgrade(&self) -> Self::Weak {
        Self::Weak {
            builder: self.builder.downgrade(),
        }
    }
}

impl glib::clone::Upgrade for DialogPruneWeak {
    type Strong = DialogPrune;

    fn upgrade(&self) -> Option<Self::Strong> {
        Some(Self::Strong {
            builder: self.builder.upgrade()?,
        })
    }
}

impl DialogPrune {
    pub fn new() -> Self {
        Self {
            builder: gtk::Builder::from_string(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/dialog_prune.ui"
            ))),
        }
    }

    fn get<T: glib::IsA<glib::object::Object>>(&self, id: &str) -> T {
        gtk::Builder::object(&self.builder, id).unwrap_or_else(|| {
            panic!(
                "Object with id '{}' not found in 'src/ui/dialog_prune.ui'",
                id
            )
        })
    }

    pub fn cancel(&self) -> gtk::Button {
        self.get("cancel")
    }

    pub fn delete(&self) -> gtk::Button {
        self.get("delete")
    }

    pub fn dialog(&self) -> adw::Window {
        self.get("dialog")
    }

    pub fn keep(&self) -> gtk::Label {
        self.get("keep")
    }

    pub fn leaflet(&self) -> adw::Leaflet {
        self.get("leaflet")
    }

    pub fn page_decision(&self) -> gtk::Box {
        self.get("page_decision")
    }

    pub fn prune(&self) -> gtk::Label {
        self.get("prune")
    }

    pub fn untouched(&self) -> gtk::Label {
        self.get("untouched")
    }
}

#[derive(Clone)]
pub struct DialogPruneReview {
    builder: gtk::Builder,
}

#[derive(Clone)]
pub struct DialogPruneReviewWeak {
    builder: glib::WeakRef<gtk::Builder>,
}

impl glib::clone::Downgrade for DialogPruneReview {
    type Weak = DialogPruneReviewWeak;

    fn downgrade(&self) -> Self::Weak {
        Self::Weak {
            builder: self.builder.downgrade(),
        }
    }
}

impl glib::clone::Upgrade for DialogPruneReviewWeak {
    type Strong = DialogPruneReview;

    fn upgrade(&self) -> Option<Self::Strong> {
        Some(Self::Strong {
            builder: self.builder.upgrade()?,
        })
    }
}

impl DialogPruneReview {
    pub fn new() -> Self {
        Self {
            builder: gtk::Builder::from_string(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/dialog_prune_review.ui"
            ))),
        }
    }

    fn get<T: glib::IsA<glib::object::Object>>(&self, id: &str) -> T {
        gtk::Builder::object(&self.builder, id).unwrap_or_else(|| {
            panic!(
                "Object with id '{}' not found in 'src/ui/dialog_prune_review.ui'",
                id
            )
        })
    }

    pub fn apply(&self) -> gtk::Button {
        self.get("apply")
    }

    pub fn dialog(&self) -> adw::Window {
        self.get("dialog")
    }

    pub fn keep(&self) -> gtk::Label {
        self.get("keep")
    }

    pub fn leaflet(&self) -> adw::Leaflet {
        self.get("leaflet")
    }

    pub fn page_decision(&self) -> gtk::Box {
        self.get("page_decision")
    }

    pub fn prune(&self) -> gtk::Label {
        self.get("prune")
    }

    pub fn untouched(&self) -> gtk::Label {
        self.get("untouched")
    }
}

#[derive(Clone)]
pub struct DialogSetup {
    builder: gtk::Builder,
}

#[derive(Clone)]
pub struct DialogSetupWeak {
    builder: glib::WeakRef<gtk::Builder>,
}

impl glib::clone::Downgrade for DialogSetup {
    type Weak = DialogSetupWeak;

    fn downgrade(&self) -> Self::Weak {
        Self::Weak {
            builder: self.builder.downgrade(),
        }
    }
}

impl glib::clone::Upgrade for DialogSetupWeak {
    type Strong = DialogSetup;

    fn upgrade(&self) -> Option<Self::Strong> {
        Some(Self::Strong {
            builder: self.builder.upgrade()?,
        })
    }
}

impl DialogSetup {
    pub fn new() -> Self {
        Self {
            builder: gtk::Builder::from_string(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/dialog_setup.ui"
            ))),
        }
    }

    fn get<T: glib::IsA<glib::object::Object>>(&self, id: &str) -> T {
        gtk::Builder::object(&self.builder, id).unwrap_or_else(|| {
            panic!(
                "Object with id '{}' not found in 'src/ui/dialog_setup.ui'",
                id
            )
        })
    }

    pub fn add_button(&self) -> gtk::Button {
        self.get("add_button")
    }

    pub fn add_local_row(&self) -> adw::ActionRow {
        self.get("add_local_row")
    }

    pub fn add_remote_row(&self) -> adw::ActionRow {
        self.get("add_remote_row")
    }

    pub fn add_repo_list(&self) -> gtk::ListBox {
        self.get("add_repo_list")
    }

    pub fn add_task(&self) -> crate::ui::export::AddConfigTask {
        self.get("add_task")
    }

    pub fn ask_password(&self) -> gtk::PasswordEntry {
        self.get("ask_password")
    }

    pub fn back_to_overview(&self) -> gtk::Button {
        self.get("back_to_overview")
    }

    pub fn button_stack(&self) -> gtk::Stack {
        self.get("button_stack")
    }

    pub fn command_line_args(&self) -> gtk::TextView {
        self.get("command_line_args")
    }

    pub fn detail_stack(&self) -> adw::ViewStack {
        self.get("detail_stack")
    }

    pub fn dialog(&self) -> adw::Window {
        self.get("dialog")
    }

    pub fn encryption(&self) -> gtk::Stack {
        self.get("encryption")
    }

    pub fn encryption_box(&self) -> adw::PreferencesGroup {
        self.get("encryption_box")
    }

    pub fn init_button(&self) -> gtk::Button {
        self.get("init_button")
    }

    pub fn init_dir(&self) -> gtk::Entry {
        self.get("init_dir")
    }

    pub fn init_local_row(&self) -> adw::ActionRow {
        self.get("init_local_row")
    }

    pub fn init_path(&self) -> crate::ui::export::FolderButton {
        self.get("init_path")
    }

    pub fn init_remote_row(&self) -> adw::ActionRow {
        self.get("init_remote_row")
    }

    pub fn init_repo_list(&self) -> gtk::ListBox {
        self.get("init_repo_list")
    }

    pub fn leaflet(&self) -> adw::Leaflet {
        self.get("leaflet")
    }

    pub fn location_local(&self) -> gtk::Box {
        self.get("location_local")
    }

    pub fn location_remote(&self) -> gtk::Box {
        self.get("location_remote")
    }

    pub fn location_stack(&self) -> gtk::Stack {
        self.get("location_stack")
    }

    pub fn location_url(&self) -> gtk::Entry {
        self.get("location_url")
    }

    pub fn non_journaling_warning(&self) -> gtk::Box {
        self.get("non_journaling_warning")
    }

    pub fn page_creating(&self) -> gtk::WindowHandle {
        self.get("page_creating")
    }

    pub fn page_detail(&self) -> gtk::Box {
        self.get("page_detail")
    }

    pub fn page_detail_default(&self) -> adw::PreferencesPage {
        self.get("page_detail_default")
    }

    pub fn page_detail_settings(&self) -> adw::PreferencesPage {
        self.get("page_detail_settings")
    }

    pub fn page_overview(&self) -> gtk::Box {
        self.get("page_overview")
    }

    pub fn page_password(&self) -> gtk::Stack {
        self.get("page_password")
    }

    pub fn page_password_continue(&self) -> gtk::Button {
        self.get("page_password_continue")
    }

    pub fn page_password_input(&self) -> gtk::Box {
        self.get("page_password_input")
    }

    pub fn page_password_pending(&self) -> gtk::WindowHandle {
        self.get("page_password_pending")
    }

    pub fn page_transfer(&self) -> gtk::Stack {
        self.get("page_transfer")
    }

    pub fn page_transfer_pending(&self) -> gtk::WindowHandle {
        self.get("page_transfer_pending")
    }

    pub fn page_transfer_prefix(&self) -> gtk::Box {
        self.get("page_transfer_prefix")
    }

    pub fn page_transfer_select(&self) -> gtk::Box {
        self.get("page_transfer_select")
    }

    pub fn password(&self) -> gtk::PasswordEntry {
        self.get("password")
    }

    pub fn password_confirm(&self) -> gtk::PasswordEntry {
        self.get("password_confirm")
    }

    pub fn password_quality(&self) -> gtk::LevelBar {
        self.get("password_quality")
    }

    pub fn pending_spinner(&self) -> gtk::Spinner {
        self.get("pending_spinner")
    }

    pub fn prefix(&self) -> gtk::Entry {
        self.get("prefix")
    }

    pub fn prefix_submit(&self) -> gtk::Button {
        self.get("prefix_submit")
    }

    pub fn show_settings(&self) -> gtk::ToggleButton {
        self.get("show_settings")
    }

    pub fn transfer_pending_spinner(&self) -> gtk::Spinner {
        self.get("transfer_pending_spinner")
    }

    pub fn transfer_suggestions(&self) -> gtk::ListBox {
        self.get("transfer_suggestions")
    }

    pub fn unencrypted(&self) -> gtk::Box {
        self.get("unencrypted")
    }
}

#[derive(Clone)]
pub struct DialogSetupTransferOption {
    builder: gtk::Builder,
}

#[derive(Clone)]
pub struct DialogSetupTransferOptionWeak {
    builder: glib::WeakRef<gtk::Builder>,
}

impl glib::clone::Downgrade for DialogSetupTransferOption {
    type Weak = DialogSetupTransferOptionWeak;

    fn downgrade(&self) -> Self::Weak {
        Self::Weak {
            builder: self.builder.downgrade(),
        }
    }
}

impl glib::clone::Upgrade for DialogSetupTransferOptionWeak {
    type Strong = DialogSetupTransferOption;

    fn upgrade(&self) -> Option<Self::Strong> {
        Some(Self::Strong {
            builder: self.builder.upgrade()?,
        })
    }
}

impl DialogSetupTransferOption {
    pub fn new() -> Self {
        Self {
            builder: gtk::Builder::from_string(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/dialog_setup_transfer_option.ui"
            ))),
        }
    }

    fn get<T: glib::IsA<glib::object::Object>>(&self, id: &str) -> T {
        gtk::Builder::object(&self.builder, id).unwrap_or_else(|| {
            panic!(
                "Object with id '{}' not found in 'src/ui/dialog_setup_transfer_option.ui'",
                id
            )
        })
    }

    pub fn exclude(&self) -> crate::ui::export::WrapBox {
        self.get("exclude")
    }

    pub fn hostname(&self) -> gtk::Label {
        self.get("hostname")
    }

    pub fn include(&self) -> crate::ui::export::WrapBox {
        self.get("include")
    }

    pub fn prefix(&self) -> gtk::Label {
        self.get("prefix")
    }

    pub fn transfer(&self) -> adw::ActionRow {
        self.get("transfer")
    }

    pub fn username(&self) -> gtk::Label {
        self.get("username")
    }

    pub fn widget(&self) -> gtk::ListBoxRow {
        self.get("widget")
    }
}

#[derive(Clone)]
pub struct DialogShortcuts {
    builder: gtk::Builder,
}

#[derive(Clone)]
pub struct DialogShortcutsWeak {
    builder: glib::WeakRef<gtk::Builder>,
}

impl glib::clone::Downgrade for DialogShortcuts {
    type Weak = DialogShortcutsWeak;

    fn downgrade(&self) -> Self::Weak {
        Self::Weak {
            builder: self.builder.downgrade(),
        }
    }
}

impl glib::clone::Upgrade for DialogShortcutsWeak {
    type Strong = DialogShortcuts;

    fn upgrade(&self) -> Option<Self::Strong> {
        Some(Self::Strong {
            builder: self.builder.upgrade()?,
        })
    }
}

impl DialogShortcuts {
    pub fn new() -> Self {
        Self {
            builder: gtk::Builder::from_string(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/dialog_shortcuts.ui"
            ))),
        }
    }

    fn get<T: glib::IsA<glib::object::Object>>(&self, id: &str) -> T {
        gtk::Builder::object(&self.builder, id).unwrap_or_else(|| {
            panic!(
                "Object with id '{}' not found in 'src/ui/dialog_shortcuts.ui'",
                id
            )
        })
    }

    pub fn dialog(&self) -> gtk::ShortcutsWindow {
        self.get("dialog")
    }
}

#[derive(Clone)]
pub struct DialogStorage {
    builder: gtk::Builder,
}

#[derive(Clone)]
pub struct DialogStorageWeak {
    builder: glib::WeakRef<gtk::Builder>,
}

impl glib::clone::Downgrade for DialogStorage {
    type Weak = DialogStorageWeak;

    fn downgrade(&self) -> Self::Weak {
        Self::Weak {
            builder: self.builder.downgrade(),
        }
    }
}

impl glib::clone::Upgrade for DialogStorageWeak {
    type Strong = DialogStorage;

    fn upgrade(&self) -> Option<Self::Strong> {
        Some(Self::Strong {
            builder: self.builder.upgrade()?,
        })
    }
}

impl DialogStorage {
    pub fn new() -> Self {
        Self {
            builder: gtk::Builder::from_string(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/dialog_storage.ui"
            ))),
        }
    }

    fn get<T: glib::IsA<glib::object::Object>>(&self, id: &str) -> T {
        gtk::Builder::object(&self.builder, id).unwrap_or_else(|| {
            panic!(
                "Object with id '{}' not found in 'src/ui/dialog_storage.ui'",
                id
            )
        })
    }

    pub fn device(&self) -> gtk::Label {
        self.get("device")
    }

    pub fn dialog(&self) -> gtk::Dialog {
        self.get("dialog")
    }

    pub fn disk(&self) -> gtk::ListBox {
        self.get("disk")
    }

    pub fn fs(&self) -> gtk::Box {
        self.get("fs")
    }

    pub fn fs_free(&self) -> gtk::Label {
        self.get("fs_free")
    }

    pub fn fs_size(&self) -> gtk::Label {
        self.get("fs_size")
    }

    pub fn fs_usage(&self) -> gtk::LevelBar {
        self.get("fs_usage")
    }

    pub fn path(&self) -> gtk::Label {
        self.get("path")
    }

    pub fn remote(&self) -> gtk::ListBox {
        self.get("remote")
    }

    pub fn uri(&self) -> gtk::Label {
        self.get("uri")
    }

    pub fn volume(&self) -> gtk::Label {
        self.get("volume")
    }
}

#[derive(Clone)]
pub struct OverviewItem {
    builder: gtk::Builder,
}

#[derive(Clone)]
pub struct OverviewItemWeak {
    builder: glib::WeakRef<gtk::Builder>,
}

impl glib::clone::Downgrade for OverviewItem {
    type Weak = OverviewItemWeak;

    fn downgrade(&self) -> Self::Weak {
        Self::Weak {
            builder: self.builder.downgrade(),
        }
    }
}

impl glib::clone::Upgrade for OverviewItemWeak {
    type Strong = OverviewItem;

    fn upgrade(&self) -> Option<Self::Strong> {
        Some(Self::Strong {
            builder: self.builder.upgrade()?,
        })
    }
}

impl OverviewItem {
    pub fn new() -> Self {
        Self {
            builder: gtk::Builder::from_string(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/ui/overview_item.ui"
            ))),
        }
    }

    fn get<T: glib::IsA<glib::object::Object>>(&self, id: &str) -> T {
        gtk::Builder::object(&self.builder, id).unwrap_or_else(|| {
            panic!(
                "Object with id '{}' not found in 'src/ui/overview_item.ui'",
                id
            )
        })
    }

    pub fn include(&self) -> crate::ui::export::WrapBox {
        self.get("include")
    }

    pub fn location(&self) -> adw::ActionRow {
        self.get("location")
    }

    pub fn location_icon(&self) -> gtk::Image {
        self.get("location_icon")
    }

    pub fn location_subtitle(&self) -> gtk::Label {
        self.get("location_subtitle")
    }

    pub fn location_title(&self) -> gtk::Label {
        self.get("location_title")
    }

    pub fn schedule(&self) -> adw::ActionRow {
        self.get("schedule")
    }

    pub fn schedule_icon(&self) -> crate::ui::export::StatusIcon {
        self.get("schedule_icon")
    }

    pub fn status(&self) -> adw::ActionRow {
        self.get("status")
    }

    pub fn status_graphic(&self) -> gtk::Stack {
        self.get("status_graphic")
    }

    pub fn status_icon(&self) -> crate::ui::export::StatusIcon {
        self.get("status_icon")
    }

    pub fn status_spinner(&self) -> gtk::Spinner {
        self.get("status_spinner")
    }

    pub fn widget(&self) -> gtk::ListBoxRow {
        self.get("widget")
    }
}
