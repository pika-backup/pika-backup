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
            panic!("Object with id '{id}' not found in 'src/ui/dialog_encryption_password.ui'")
        })
    }

    pub fn dialog(&self) -> adw::MessageDialog {
        self.get("dialog")
    }

    pub fn password(&self) -> gtk::PasswordEntry {
        self.get("password")
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
            panic!("Object with id '{id}' not found in 'src/ui/dialog_setup.ui'")
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

    pub fn button_stack(&self) -> gtk::Stack {
        self.get("button_stack")
    }

    pub fn command_line_args_entry(&self) -> adw::EntryRow {
        self.get("command_line_args_entry")
    }

    pub fn creating_repository_spinner(&self) -> gtk::Spinner {
        self.get("creating_repository_spinner")
    }

    pub fn dialog(&self) -> adw::Window {
        self.get("dialog")
    }

    pub fn encryption_preferences_group(&self) -> crate::ui::export::EncryptionPreferencesGroup {
        self.get("encryption_preferences_group")
    }

    pub fn init_button(&self) -> gtk::Button {
        self.get("init_button")
    }

    pub fn init_dir(&self) -> adw::EntryRow {
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

    pub fn location_group_local(&self) -> adw::PreferencesGroup {
        self.get("location_group_local")
    }

    pub fn location_group_remote(&self) -> adw::PreferencesGroup {
        self.get("location_group_remote")
    }

    pub fn location_local(&self) -> adw::ActionRow {
        self.get("location_local")
    }

    pub fn location_url(&self) -> adw::EntryRow {
        self.get("location_url")
    }

    pub fn navigation_view(&self) -> adw::NavigationView {
        self.get("navigation_view")
    }

    pub fn non_journaling_warning(&self) -> gtk::Box {
        self.get("non_journaling_warning")
    }

    pub fn page_creating(&self) -> adw::NavigationPage {
        self.get("page_creating")
    }

    pub fn page_detail(&self) -> adw::NavigationPage {
        self.get("page_detail")
    }

    pub fn page_detail_continue(&self) -> gtk::Button {
        self.get("page_detail_continue")
    }

    pub fn page_overview(&self) -> adw::NavigationPage {
        self.get("page_overview")
    }

    pub fn page_password(&self) -> adw::NavigationPage {
        self.get("page_password")
    }

    pub fn page_password_continue(&self) -> gtk::Button {
        self.get("page_password_continue")
    }

    pub fn page_password_input(&self) -> adw::ToolbarView {
        self.get("page_password_input")
    }

    pub fn page_password_pending(&self) -> gtk::WindowHandle {
        self.get("page_password_pending")
    }

    pub fn page_password_stack(&self) -> gtk::Stack {
        self.get("page_password_stack")
    }

    pub fn page_setup_encryption(&self) -> adw::NavigationPage {
        self.get("page_setup_encryption")
    }

    pub fn page_transfer(&self) -> adw::NavigationPage {
        self.get("page_transfer")
    }

    pub fn page_transfer_pending(&self) -> adw::ToolbarView {
        self.get("page_transfer_pending")
    }

    pub fn page_transfer_prefix(&self) -> adw::NavigationPage {
        self.get("page_transfer_prefix")
    }

    pub fn page_transfer_select(&self) -> adw::ToolbarView {
        self.get("page_transfer_select")
    }

    pub fn page_transfer_stack(&self) -> gtk::Stack {
        self.get("page_transfer_stack")
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
            panic!("Object with id '{id}' not found in 'src/ui/dialog_setup_transfer_option.ui'")
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
            panic!("Object with id '{id}' not found in 'src/ui/overview_item.ui'")
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

    pub fn schedule(&self) -> crate::ui::export::StatusRow {
        self.get("schedule")
    }

    pub fn status(&self) -> crate::ui::export::StatusRow {
        self.get("status")
    }

    pub fn widget(&self) -> gtk::ListBoxRow {
        self.get("widget")
    }
}
