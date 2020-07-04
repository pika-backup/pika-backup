pub struct About {
    builder: gtk::Builder,
}

impl About {
    pub fn new() -> Self {
        Self {
            builder: gtk::Builder::from_string(include_str!(concat!(data_dir!(), "/ui/about.ui"))),
        }
    }

    fn get<T: glib::IsA<glib::object::Object>>(&self, id: &str) -> T {
        gtk::prelude::BuilderExtManual::get_object(&self.builder, id)
            .unwrap_or_else(|| panic!("Object with id '{}' not found in 'ui/about.ui'", id))
    }

    pub fn dialog(&self) -> gtk::AboutDialog {
        self.get("dialog")
    }
}

pub struct EncryptionPassword {
    builder: gtk::Builder,
}

impl EncryptionPassword {
    pub fn new() -> Self {
        Self {
            builder: gtk::Builder::from_string(include_str!(concat!(
                data_dir!(),
                "/ui/encryption_password.ui"
            ))),
        }
    }

    fn get<T: glib::IsA<glib::object::Object>>(&self, id: &str) -> T {
        gtk::prelude::BuilderExtManual::get_object(&self.builder, id).unwrap_or_else(|| {
            panic!(
                "Object with id '{}' not found in 'ui/encryption_password.ui'",
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

    pub fn add_pending_label(&self) -> gtk::Label {
        self.get("add_pending_label")
    }

    pub fn archive_comment(&self) -> gtk::Label {
        self.get("archive_comment")
    }

    pub fn archive_end(&self) -> gtk::Label {
        self.get("archive_end")
    }

    pub fn archive_hostname(&self) -> gtk::Label {
        self.get("archive_hostname")
    }

    pub fn archive_list(&self) -> gtk::ListBox {
        self.get("archive_list")
    }

    pub fn archive_name(&self) -> gtk::Label {
        self.get("archive_name")
    }

    pub fn archive_popover(&self) -> gtk::Popover {
        self.get("archive_popover")
    }

    pub fn archive_progress(&self) -> gtk::ProgressBar {
        self.get("archive_progress")
    }

    pub fn archive_scrolled(&self) -> gtk::ScrolledWindow {
        self.get("archive_scrolled")
    }

    pub fn archive_start(&self) -> gtk::Label {
        self.get("archive_start")
    }

    pub fn archive_username(&self) -> gtk::Label {
        self.get("archive_username")
    }

    pub fn backup_exclude(&self) -> gtk::ListBox {
        self.get("backup_exclude")
    }

    pub fn backup_run(&self) -> gtk::Button {
        self.get("backup_run")
    }

    pub fn backup_status_popover(&self) -> gtk::Popover {
        self.get("backup_status_popover")
    }

    pub fn browse_archive(&self) -> gtk::Button {
        self.get("browse_archive")
    }

    pub fn current_path(&self) -> gtk::Label {
        self.get("current_path")
    }

    pub fn deduplicated_size(&self) -> gtk::Label {
        self.get("deduplicated_size")
    }

    pub fn delete_backup_conf(&self) -> gtk::ModelButton {
        self.get("delete_backup_conf")
    }

    pub fn detail_device_not_connected(&self) -> gtk::InfoBar {
        self.get("detail_device_not_connected")
    }

    pub fn detail_menu(&self) -> gtk::MenuButton {
        self.get("detail_menu")
    }

    pub fn detail_menu_popover(&self) -> gtk::PopoverMenu {
        self.get("detail_menu_popover")
    }

    pub fn detail_scrolled(&self) -> gtk::Viewport {
        self.get("detail_scrolled")
    }

    pub fn error_message(&self) -> gtk::Label {
        self.get("error_message")
    }

    pub fn home_icon(&self) -> gtk::Image {
        self.get("home_icon")
    }

    pub fn include(&self) -> gtk::ListBox {
        self.get("include")
    }

    pub fn include_home(&self) -> gtk::Switch {
        self.get("include_home")
    }

    pub fn main_backups(&self) -> gtk::ListBox {
        self.get("main_backups")
    }

    pub fn main_menu(&self) -> gtk::MenuButton {
        self.get("main_menu")
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

    pub fn overview_empty(&self) -> gtk::Box {
        self.get("overview_empty")
    }

    pub fn overview_none_empty(&self) -> gtk::Label {
        self.get("overview_none_empty")
    }

    pub fn page_archives(&self) -> gtk::Box {
        self.get("page_archives")
    }

    pub fn page_detail(&self) -> gtk::Box {
        self.get("page_detail")
    }

    pub fn page_overview(&self) -> gtk::ScrolledWindow {
        self.get("page_overview")
    }

    pub fn page_pending(&self) -> gtk::Box {
        self.get("page_pending")
    }

    pub fn pending_menu(&self) -> gtk::MenuButton {
        self.get("pending_menu")
    }

    pub fn pending_popover(&self) -> gtk::Popover {
        self.get("pending_popover")
    }

    pub fn percent_message(&self) -> gtk::Label {
        self.get("percent_message")
    }

    pub fn previous(&self) -> gtk::Button {
        self.get("previous")
    }

    pub fn progress(&self) -> gtk::ProgressBar {
        self.get("progress")
    }

    pub fn stack(&self) -> gtk::Stack {
        self.get("stack")
    }

    pub fn status_button(&self) -> gtk::MenuButton {
        self.get("status_button")
    }

    pub fn status_icon(&self) -> gtk::Stack {
        self.get("status_icon")
    }

    pub fn status_subtext(&self) -> gtk::Label {
        self.get("status_subtext")
    }

    pub fn status_text(&self) -> gtk::Label {
        self.get("status_text")
    }

    pub fn stop_backup_create(&self) -> gtk::Button {
        self.get("stop_backup_create")
    }

    pub fn target_listbox(&self) -> gtk::ListBox {
        self.get("target_listbox")
    }

    pub fn window(&self) -> gtk::ApplicationWindow {
        self.get("window")
    }
}

pub struct NewBackup {
    builder: gtk::Builder,
}

impl NewBackup {
    pub fn new() -> Self {
        Self {
            builder: gtk::Builder::from_string(include_str!(concat!(
                data_dir!(),
                "/ui/new_backup.ui"
            ))),
        }
    }

    fn get<T: glib::IsA<glib::object::Object>>(&self, id: &str) -> T {
        gtk::prelude::BuilderExtManual::get_object(&self.builder, id)
            .unwrap_or_else(|| panic!("Object with id '{}' not found in 'ui/new_backup.ui'", id))
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

pub struct Storage {
    builder: gtk::Builder,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            builder: gtk::Builder::from_string(include_str!(concat!(
                data_dir!(),
                "/ui/storage.ui"
            ))),
        }
    }

    fn get<T: glib::IsA<glib::object::Object>>(&self, id: &str) -> T {
        gtk::prelude::BuilderExtManual::get_object(&self.builder, id)
            .unwrap_or_else(|| panic!("Object with id '{}' not found in 'ui/storage.ui'", id))
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
