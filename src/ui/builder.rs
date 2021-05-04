pub struct DialogAbout {
    builder: gtk::Builder,
}

impl DialogAbout {
    pub fn new() -> Self {
        Self {
            builder: gtk::Builder::from_string(include_str!(concat!(
                data_dir!(),
                "/ui/dialog_about.ui"
            ))),
        }
    }

    fn get<T: glib::IsA<glib::object::Object>>(&self, id: &str) -> T {
        gtk::prelude::BuilderExtManual::object(&self.builder, id)
            .unwrap_or_else(|| panic!("Object with id '{}' not found in 'ui/dialog_about.ui'", id))
    }

    pub fn dialog(&self) -> gtk::AboutDialog {
        self.get("dialog")
    }
}

pub struct DialogAddConfig {
    builder: gtk::Builder,
}

impl DialogAddConfig {
    pub fn new() -> Self {
        Self {
            builder: gtk::Builder::from_string(include_str!(concat!(
                data_dir!(),
                "/ui/dialog_add_config.ui"
            ))),
        }
    }

    fn get<T: glib::IsA<glib::object::Object>>(&self, id: &str) -> T {
        gtk::prelude::BuilderExtManual::object(&self.builder, id).unwrap_or_else(|| {
            panic!(
                "Object with id '{}' not found in 'ui/dialog_add_config.ui'",
                id
            )
        })
    }

    pub fn add_button(&self) -> gtk::Button {
        self.get("add_button")
    }

    pub fn add_local_row(&self) -> libhandy::ActionRow {
        self.get("add_local_row")
    }

    pub fn add_remote_row(&self) -> libhandy::ActionRow {
        self.get("add_remote_row")
    }

    pub fn add_repo_list(&self) -> gtk::ListBox {
        self.get("add_repo_list")
    }

    pub fn button_stack(&self) -> gtk::Stack {
        self.get("button_stack")
    }

    pub fn cancel_button(&self) -> gtk::Button {
        self.get("cancel_button")
    }

    pub fn command_line_args(&self) -> gtk::TextView {
        self.get("command_line_args")
    }

    pub fn dialog(&self) -> gtk::Dialog {
        self.get("dialog")
    }

    pub fn dialog_vbox(&self) -> gtk::Box {
        self.get("dialog_vbox")
    }

    pub fn encryption(&self) -> gtk::Stack {
        self.get("encryption")
    }

    pub fn encryption_box(&self) -> gtk::Box {
        self.get("encryption_box")
    }

    pub fn existing_repos(&self) -> gtk::Box {
        self.get("existing_repos")
    }

    pub fn init_button(&self) -> gtk::Button {
        self.get("init_button")
    }

    pub fn init_dir(&self) -> gtk::Entry {
        self.get("init_dir")
    }

    pub fn init_local_row(&self) -> libhandy::ActionRow {
        self.get("init_local_row")
    }

    pub fn init_path(&self) -> gtk::FileChooserButton {
        self.get("init_path")
    }

    pub fn init_remote_row(&self) -> libhandy::ActionRow {
        self.get("init_remote_row")
    }

    pub fn init_repo_list(&self) -> gtk::ListBox {
        self.get("init_repo_list")
    }

    pub fn label1(&self) -> gtk::Label {
        self.get("label1")
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

    pub fn location_url_help(&self) -> gtk::Popover {
        self.get("location_url_help")
    }

    pub fn new_page(&self) -> gtk::ScrolledWindow {
        self.get("new_page")
    }

    pub fn no_button(&self) -> gtk::Box {
        self.get("no_button")
    }

    pub fn non_journaling_warning(&self) -> gtk::Box {
        self.get("non_journaling_warning")
    }

    pub fn password(&self) -> gtk::Entry {
        self.get("password")
    }

    pub fn password_confirm(&self) -> gtk::Entry {
        self.get("password_confirm")
    }

    pub fn password_quality(&self) -> gtk::LevelBar {
        self.get("password_quality")
    }

    pub fn password_store(&self) -> gtk::CheckButton {
        self.get("password_store")
    }

    pub fn spacer_1(&self) -> gtk::Box {
        self.get("spacer_1")
    }

    pub fn stack(&self) -> gtk::Stack {
        self.get("stack")
    }

    pub fn stackswitcher1(&self) -> gtk::StackSwitcher {
        self.get("stackswitcher1")
    }

    pub fn unencrypted(&self) -> gtk::Box {
        self.get("unencrypted")
    }

    pub fn x(&self) -> gtk::SizeGroup {
        self.get("x")
    }

    pub fn y(&self) -> gtk::SizeGroup {
        self.get("y")
    }
}

pub struct DialogDeviceMissing {
    builder: gtk::Builder,
}

impl DialogDeviceMissing {
    pub fn new() -> Self {
        Self {
            builder: gtk::Builder::from_string(include_str!(concat!(
                data_dir!(),
                "/ui/dialog_device_missing.ui"
            ))),
        }
    }

    fn get<T: glib::IsA<glib::object::Object>>(&self, id: &str) -> T {
        gtk::prelude::BuilderExtManual::object(&self.builder, id).unwrap_or_else(|| {
            panic!(
                "Object with id '{}' not found in 'ui/dialog_device_missing.ui'",
                id
            )
        })
    }

    pub fn cancel(&self) -> gtk::Button {
        self.get("cancel")
    }

    pub fn icon(&self) -> gtk::Box {
        self.get("icon")
    }

    pub fn name(&self) -> gtk::Label {
        self.get("name")
    }

    pub fn purpose(&self) -> gtk::Label {
        self.get("purpose")
    }

    pub fn window(&self) -> gtk::Dialog {
        self.get("window")
    }
}

pub struct DialogEncryptionPassword {
    builder: gtk::Builder,
}

impl DialogEncryptionPassword {
    pub fn new() -> Self {
        Self {
            builder: gtk::Builder::from_string(include_str!(concat!(
                data_dir!(),
                "/ui/dialog_encryption_password.ui"
            ))),
        }
    }

    fn get<T: glib::IsA<glib::object::Object>>(&self, id: &str) -> T {
        gtk::prelude::BuilderExtManual::object(&self.builder, id).unwrap_or_else(|| {
            panic!(
                "Object with id '{}' not found in 'ui/dialog_encryption_password.ui'",
                id
            )
        })
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

    pub fn password(&self) -> gtk::Entry {
        self.get("password")
    }

    pub fn password_forget(&self) -> gtk::RadioButton {
        self.get("password_forget")
    }

    pub fn password_store(&self) -> gtk::RadioButton {
        self.get("password_store")
    }

    pub fn purpose(&self) -> gtk::Label {
        self.get("purpose")
    }
}

pub struct DialogStorage {
    builder: gtk::Builder,
}

impl DialogStorage {
    pub fn new() -> Self {
        Self {
            builder: gtk::Builder::from_string(include_str!(concat!(
                data_dir!(),
                "/ui/dialog_storage.ui"
            ))),
        }
    }

    fn get<T: glib::IsA<glib::object::Object>>(&self, id: &str) -> T {
        gtk::prelude::BuilderExtManual::object(&self.builder, id).unwrap_or_else(|| {
            panic!(
                "Object with id '{}' not found in 'ui/dialog_storage.ui'",
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

    pub fn disk(&self) -> gtk::Grid {
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

    pub fn label1(&self) -> gtk::Label {
        self.get("label1")
    }

    pub fn label2(&self) -> gtk::Label {
        self.get("label2")
    }

    pub fn label3(&self) -> gtk::Label {
        self.get("label3")
    }

    pub fn label4(&self) -> gtk::Label {
        self.get("label4")
    }

    pub fn label5(&self) -> gtk::Label {
        self.get("label5")
    }

    pub fn path(&self) -> gtk::Label {
        self.get("path")
    }

    pub fn remote(&self) -> gtk::Grid {
        self.get("remote")
    }

    pub fn uri(&self) -> gtk::Label {
        self.get("uri")
    }

    pub fn volume(&self) -> gtk::Label {
        self.get("volume")
    }
}

pub struct Main {
    builder: gtk::Builder,
}

impl Main {
    pub fn new() -> Self {
        Self {
            builder: gtk::Builder::from_string(include_str!(concat!(data_dir!(), "/ui/main.ui"))),
        }
    }

    fn get<T: glib::IsA<glib::object::Object>>(&self, id: &str) -> T {
        gtk::prelude::BuilderExtManual::object(&self.builder, id)
            .unwrap_or_else(|| panic!("Object with id '{}' not found in 'ui/main.ui'", id))
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

    pub fn add_pending_label(&self) -> gtk::Label {
        self.get("add_pending_label")
    }

    pub fn archive_list(&self) -> gtk::ListBox {
        self.get("archive_list")
    }

    pub fn archive_list_placeholder(&self) -> gtk::Label {
        self.get("archive_list_placeholder")
    }

    pub fn archives_eject_button(&self) -> gtk::Button {
        self.get("archives_eject_button")
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

    pub fn detail_dialog_vbox(&self) -> gtk::Box {
        self.get("detail_dialog_vbox")
    }

    pub fn detail_exclude_placeholder(&self) -> gtk::Label {
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

    pub fn detail_path_row(&self) -> libhandy::ActionRow {
        self.get("detail_path_row")
    }

    pub fn detail_repo_icon(&self) -> gtk::Image {
        self.get("detail_repo_icon")
    }

    pub fn detail_repo_row(&self) -> libhandy::ActionRow {
        self.get("detail_repo_row")
    }

    pub fn detail_running_backup_info(&self) -> gtk::Dialog {
        self.get("detail_running_backup_info")
    }

    pub fn detail_stack(&self) -> gtk::Stack {
        self.get("detail_stack")
    }

    pub fn detail_stats(&self) -> gtk::ListBox {
        self.get("detail_stats")
    }

    pub fn detail_status_right(&self) -> gtk::Box {
        self.get("detail_status_right")
    }

    pub fn detail_status_row(&self) -> libhandy::ActionRow {
        self.get("detail_status_row")
    }

    pub fn include(&self) -> gtk::ListBox {
        self.get("include")
    }

    pub fn include_home(&self) -> gtk::Switch {
        self.get("include_home")
    }

    pub fn include_home_row(&self) -> libhandy::ActionRow {
        self.get("include_home_row")
    }

    pub fn internal_message(&self) -> gtk::InfoBar {
        self.get("internal_message")
    }

    pub fn internal_message_text(&self) -> gtk::Label {
        self.get("internal_message_text")
    }

    pub fn main_backups(&self) -> gtk::ListBox {
        self.get("main_backups")
    }

    pub fn main_menu_popover(&self) -> gtk::PopoverMenu {
        self.get("main_menu_popover")
    }

    pub fn main_stack(&self) -> gtk::Stack {
        self.get("main_stack")
    }

    pub fn page_archives(&self) -> gtk::ScrolledWindow {
        self.get("page_archives")
    }

    pub fn page_backup(&self) -> gtk::ScrolledWindow {
        self.get("page_backup")
    }

    pub fn page_detail(&self) -> gtk::Box {
        self.get("page_detail")
    }

    pub fn page_overview(&self) -> gtk::ScrolledWindow {
        self.get("page_overview")
    }

    pub fn page_overview_empty(&self) -> gtk::Box {
        self.get("page_overview_empty")
    }

    pub fn page_pending(&self) -> gtk::Box {
        self.get("page_pending")
    }

    pub fn page_pending_spinner(&self) -> gtk::Spinner {
        self.get("page_pending_spinner")
    }

    pub fn pending_menu(&self) -> gtk::MenuButton {
        self.get("pending_menu")
    }

    pub fn pending_menu_spinner(&self) -> gtk::Spinner {
        self.get("pending_menu_spinner")
    }

    pub fn pending_popover(&self) -> gtk::Popover {
        self.get("pending_popover")
    }

    pub fn primary_menu_button(&self) -> gtk::MenuButton {
        self.get("primary_menu_button")
    }

    pub fn refresh_archives(&self) -> gtk::Button {
        self.get("refresh_archives")
    }

    pub fn remove_backup(&self) -> gtk::ModelButton {
        self.get("remove_backup")
    }

    pub fn secondary_menu_button(&self) -> gtk::MenuButton {
        self.get("secondary_menu_button")
    }

    pub fn secondary_menu_popover(&self) -> gtk::Popover {
        self.get("secondary_menu_popover")
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

    pub fn target_listbox(&self) -> gtk::ListBox {
        self.get("target_listbox")
    }

    pub fn view_switcher_bottom(&self) -> libhandy::ViewSwitcherBar {
        self.get("view_switcher_bottom")
    }

    pub fn view_switcher_title(&self) -> libhandy::ViewSwitcherTitle {
        self.get("view_switcher_title")
    }

    pub fn viewport_archives(&self) -> gtk::Viewport {
        self.get("viewport_archives")
    }

    pub fn viewport_detail(&self) -> gtk::Viewport {
        self.get("viewport_detail")
    }

    pub fn window(&self) -> libhandy::ApplicationWindow {
        self.get("window")
    }
}

pub struct OverviewItem {
    builder: gtk::Builder,
}

impl OverviewItem {
    pub fn new() -> Self {
        Self {
            builder: gtk::Builder::from_string(include_str!(concat!(
                data_dir!(),
                "/ui/overview_item.ui"
            ))),
        }
    }

    fn get<T: glib::IsA<glib::object::Object>>(&self, id: &str) -> T {
        gtk::prelude::BuilderExtManual::object(&self.builder, id)
            .unwrap_or_else(|| panic!("Object with id '{}' not found in 'ui/overview_item.ui'", id))
    }

    pub fn include(&self) -> gtk::Box {
        self.get("include")
    }

    pub fn location(&self) -> gtk::Label {
        self.get("location")
    }

    pub fn location_icon(&self) -> gtk::Image {
        self.get("location_icon")
    }

    pub fn status(&self) -> gtk::Label {
        self.get("status")
    }

    pub fn status_area(&self) -> gtk::Grid {
        self.get("status_area")
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

    pub fn substatus(&self) -> gtk::Label {
        self.get("substatus")
    }

    pub fn widget(&self) -> gtk::Box {
        self.get("widget")
    }
}
