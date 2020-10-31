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
        gtk::prelude::BuilderExtManual::get_object(&self.builder, id)
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
        gtk::prelude::BuilderExtManual::get_object(&self.builder, id).unwrap_or_else(|| {
            panic!(
                "Object with id '{}' not found in 'ui/dialog_add_config.ui'",
                id
            )
        })
    }

    pub fn add_button(&self) -> gtk::Button {
        self.get("add_button")
    }

    pub fn add_remote_page(&self) -> gtk::Box {
        self.get("add_remote_page")
    }

    pub fn add_remote_uri(&self) -> gtk::Entry {
        self.get("add_remote_uri")
    }

    pub fn add_repo_list(&self) -> gtk::ListBox {
        self.get("add_repo_list")
    }

    pub fn cancel_button(&self) -> gtk::Button {
        self.get("cancel_button")
    }

    pub fn encryption(&self) -> gtk::Stack {
        self.get("encryption")
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

    pub fn init_local(&self) -> gtk::Box {
        self.get("init_local")
    }

    pub fn init_location(&self) -> gtk::Stack {
        self.get("init_location")
    }

    pub fn init_page(&self) -> gtk::Box {
        self.get("init_page")
    }

    pub fn init_path(&self) -> gtk::FileChooserButton {
        self.get("init_path")
    }

    pub fn init_remote(&self) -> gtk::Box {
        self.get("init_remote")
    }

    pub fn init_repo_list(&self) -> gtk::ListBox {
        self.get("init_repo_list")
    }

    pub fn init_url(&self) -> gtk::Entry {
        self.get("init_url")
    }

    pub fn label1(&self) -> gtk::Label {
        self.get("label1")
    }

    pub fn new_backup(&self) -> gtk::Dialog {
        self.get("new_backup")
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
        gtk::prelude::BuilderExtManual::get_object(&self.builder, id).unwrap_or_else(|| {
            panic!(
                "Object with id '{}' not found in 'ui/dialog_device_missing.ui'",
                id
            )
        })
    }

    pub fn cancel(&self) -> gtk::Button {
        self.get("cancel")
    }

    pub fn device(&self) -> gtk::Label {
        self.get("device")
    }

    pub fn icon(&self) -> gtk::Box {
        self.get("icon")
    }

    pub fn mount(&self) -> gtk::Label {
        self.get("mount")
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
        gtk::prelude::BuilderExtManual::get_object(&self.builder, id).unwrap_or_else(|| {
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
        gtk::prelude::BuilderExtManual::get_object(&self.builder, id).unwrap_or_else(|| {
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
        gtk::prelude::BuilderExtManual::get_object(&self.builder, id)
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

    pub fn archive_progress(&self) -> gtk::ProgressBar {
        self.get("archive_progress")
    }

    pub fn archives_reloading_row(&self) -> gtk::ListBoxRow {
        self.get("archives_reloading_row")
    }

    pub fn archives_reloading_spinner(&self) -> gtk::Spinner {
        self.get("archives_reloading_spinner")
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

    pub fn current_path(&self) -> gtk::Label {
        self.get("current_path")
    }

    pub fn deduplicated_size(&self) -> gtk::Label {
        self.get("deduplicated_size")
    }

    pub fn detail_exclude_placeholder(&self) -> gtk::Label {
        self.get("detail_exclude_placeholder")
    }

    pub fn detail_exclude_stack(&self) -> gtk::Stack {
        self.get("detail_exclude_stack")
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

    pub fn detail_status_row(&self) -> libhandy::ActionRow {
        self.get("detail_status_row")
    }

    pub fn error_message(&self) -> gtk::Label {
        self.get("error_message")
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

    pub fn main_backups(&self) -> gtk::ListBox {
        self.get("main_backups")
    }

    pub fn main_menu_popover(&self) -> gtk::PopoverMenu {
        self.get("main_menu_popover")
    }

    pub fn main_stack(&self) -> gtk::Stack {
        self.get("main_stack")
    }

    pub fn message(&self) -> gtk::Label {
        self.get("message")
    }

    pub fn original_size(&self) -> gtk::Label {
        self.get("original_size")
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

    pub fn percent_message(&self) -> gtk::Label {
        self.get("percent_message")
    }

    pub fn primary_menu_button(&self) -> gtk::MenuButton {
        self.get("primary_menu_button")
    }

    pub fn progress(&self) -> gtk::ProgressBar {
        self.get("progress")
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

    pub fn stack(&self) -> gtk::Stack {
        self.get("stack")
    }

    pub fn status_icon(&self) -> gtk::Stack {
        self.get("status_icon")
    }

    pub fn status_icon_spinner(&self) -> gtk::Spinner {
        self.get("status_icon_spinner")
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
