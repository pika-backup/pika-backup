use adw::prelude::*;
use adw::subclass::prelude::*;

use super::SetupRepoLocation;
use super::types::*;
use crate::prelude::*;
use crate::widget::DialogPage;

mod imp {
    use std::cell::{Cell, RefCell};
    use std::marker::PhantomData;
    use std::sync::OnceLock;

    use gettextrs::gettext;
    use glib::WeakRef;
    use glib::subclass::Signal;

    use super::*;
    use crate::error::HandleError;
    use crate::widget::PkDialogPageImpl;
    use crate::widget::folder_row::FolderRow;
    use crate::widget::setup::advanced_options::SetupAdvancedOptionsPage;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[template(file = "location.ui")]
    #[properties(wrapper_type = super::SetupLocationPage)]
    pub struct SetupLocationPage {
        #[property(get, set = Self::set_action, builder(SetupAction::Init))]
        action: Cell<SetupAction>,
        #[property(get, set = Self::set_repo_kind, builder(SetupLocationKind::Local))]
        location_kind: Cell<SetupLocationKind>,
        #[property(get, set)]
        navigation_view: WeakRef<adw::NavigationView>,
        #[property(get)]
        advanced_options_subtitle: RefCell<String>,
        #[property(get = Self::can_continue)]
        can_continue: PhantomData<bool>,

        #[template_child]
        location_local_group_focus: TemplateChild<gtk::EventControllerFocus>,
        #[template_child]
        location_folder_row_focus: TemplateChild<gtk::EventControllerFocus>,
        #[template_child]
        location_remote_group_focus: TemplateChild<gtk::EventControllerFocus>,
        #[template_child]
        location_url_focus: TemplateChild<gtk::EventControllerFocus>,

        #[template_child]
        advanced_options_page: TemplateChild<SetupAdvancedOptionsPage>,
        #[template_child]
        pub(super) location_group_local: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) location_folder_row: TemplateChild<FolderRow>,
        #[template_child]
        pub(super) init_dir: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) non_journaling_warning: TemplateChild<gtk::Widget>,
        #[template_child]
        pub(super) location_group_remote: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) location_url: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) button_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) add_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) continue_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SetupLocationPage {
        const NAME: &'static str = "PkSetupLocationPage";
        type Type = super::SetupLocationPage;
        type ParentType = DialogPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for SetupLocationPage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("continue")
                        .param_types([
                            SetupRepoLocation::static_type(),
                            SetupCommandLineArgs::static_type(),
                        ])
                        .build(),
                ]
            })
        }

        fn constructed(&self) {
            self.parent_constructed();
            self.on_command_line_args_changed();

            if let Some(delegate) = self.init_dir.delegate() {
                delegate.update_property(&[gtk::accessible::Property::Required(true)]);
            }

            if let Some(delegate) = self.location_url.delegate() {
                delegate.update_property(&[gtk::accessible::Property::Required(true)]);

                let target =
                    gtk::DropTarget::new(glib::GString::static_type(), gtk::gdk::DragAction::COPY);
                target.set_preload(true);

                target.connect_value_notify(|target| {
                    if let Some(value) = target.value()
                        && Self::path_to_network_uri(&value).is_some()
                    {
                        // we handle this
                        return;
                    }

                    target.reject();
                });

                target.connect_drop(glib::clone!(
                    #[weak(rename_to = obj)]
                    self.obj(),
                    #[upgrade_or]
                    false,
                    move |_target, value, _x, _y| {
                        // Try to translate the a dropped file URL to a GVFS uri
                        if let Some(uri) = Self::path_to_network_uri(value) {
                            obj.imp().location_url.set_text(&uri.to_str());
                            return true;
                        }

                        false
                    },
                ));

                delegate.add_controller(target);
            }
        }
    }
    impl WidgetImpl for SetupLocationPage {}
    impl NavigationPageImpl for SetupLocationPage {
        fn shown(&self) {
            self.parent_shown();
            self.on_folder_changed();

            match self.location_kind.get() {
                SetupLocationKind::Local => {
                    if self.location_folder_row.file().is_none() {
                        self.location_folder_row.grab_focus();
                    } else {
                        self.init_dir.grab_focus();
                    }
                }
                SetupLocationKind::Remote => {
                    self.location_url.grab_focus();
                }
            }
        }
    }
    impl PkDialogPageImpl for SetupLocationPage {}

    #[gtk::template_callbacks]
    impl SetupLocationPage {
        fn path_to_network_uri(value: &glib::Value) -> Option<glib::Uri> {
            if let Ok(string) = value.get::<&str>() {
                let file = gio::File::for_path(string);
                let uri = file.uri();
                if !uri.is_empty() {
                    // We dropped a file
                    if let Ok(uri) = glib::Uri::parse(&uri, glib::UriFlags::NON_DNS) {
                        return match &*uri.scheme() {
                            "file" | "trash" | "recent" => None,
                            _ => Some(uri),
                        };
                    }
                }
            }

            None
        }

        fn emit_continue(&self, repo: SetupRepoLocation, args: SetupCommandLineArgs) {
            self.obj().emit_by_name::<()>("continue", &[&repo, &args]);
        }

        #[template_callback]
        fn push_advanced_options(&self) {
            if let Some(view) = self.navigation_view.upgrade() {
                view.push(&*self.advanced_options_page);
            }
        }

        #[template_callback]
        fn on_click_borg_server_info(&self) {
            if let Err(err) = gio::AppInfo::launch_default_for_uri(
                "help:pika-backup/setup-remote",
                Some(&self.obj().display().app_launch_context()),
            ) {
                tracing::error!("Failed to launch help: {err}");
            }
        }

        #[template_callback]
        fn on_click_network_share_info(&self) {
            if let Err(err) = gio::AppInfo::launch_default_for_uri(
                "help:pika-backup/setup-gvfs",
                Some(&self.obj().display().app_launch_context()),
            ) {
                tracing::error!("Failed to launch help: {err}");
            }
        }

        #[template_callback]
        fn on_folder_changed(&self) {
            if let Some(path) = self.location_folder_row.file().and_then(|x| x.path()) {
                self.location_folder_row
                    .set_title(&gettext("Repository Folder"));

                let mount_entry = gio::UnixMountEntry::for_file_path(path);
                match mount_entry.0.map(|x| x.fs_type()) {
                    Some(fs) => {
                        tracing::debug!("Selected filesystem type {}", fs);
                        self.non_journaling_warning.set_visible(
                            crate::NON_JOURNALING_FILESYSTEMS.iter().any(|x| x == &fs),
                        );
                    }
                    _ => {
                        self.non_journaling_warning.set_visible(false);
                    }
                }
            } else {
                self.location_folder_row
                    .set_title(&gettext("Choose Repository Folder"));

                self.non_journaling_warning.set_visible(false);
            }

            self.obj().notify_can_continue();
        }

        #[template_callback]
        fn on_command_line_args_changed(&self) {
            let args = self.advanced_options_page.command_line_args();
            let subtitle = if args.is_empty() {
                gettext("Additional command line arguments")
            } else {
                format!("<tt>{}</tt>", glib::markup_escape_text(&args.to_string()))
            };

            if subtitle != self.advanced_options_subtitle.replace(subtitle.clone()) {
                self.obj().notify_advanced_options_subtitle();
            }
        }

        pub(super) fn reset(&self) {
            self.location_folder_row.reset();
            self.location_url.set_text("");
        }

        fn set_action(&self, action: SetupAction) {
            match action {
                SetupAction::Init => {
                    self.obj()
                        .set_default_widget(Some(self.continue_button.clone()));

                    self.button_stack
                        .set_visible_child(&*(self.continue_button));
                    self.init_dir.set_text(&format!(
                        "backup-{}-{}",
                        glib::host_name(),
                        glib::user_name().to_string_lossy()
                    ));
                }
                SetupAction::AddExisting => {
                    self.obj().set_default_widget(Some(self.add_button.clone()));
                    self.button_stack.set_visible_child(&*(self.add_button));
                    self.init_dir.set_text("");
                }
            };
        }

        fn set_repo_kind(&self, repo_kind: SetupLocationKind) {
            self.location_group_local
                .set_visible(repo_kind == SetupLocationKind::Local);
            self.location_group_remote
                .set_visible(repo_kind == SetupLocationKind::Remote);

            self.location_kind.replace(repo_kind);
            self.obj().notify_can_continue();
        }

        fn can_continue(&self) -> bool {
            self.selected_location().is_ok()
        }

        fn try_continue(&self) -> Result<()> {
            let repo_location = self.selected_location()?;
            let command_line_args = self.advanced_options_page.command_line_args();

            tracing::debug!("Continue with repo location '{}'", repo_location);

            self.emit_continue(repo_location, command_line_args);

            Ok(())
        }

        #[template_callback]
        async fn on_add_button(&self) {
            self.try_continue().handle_transient_for(&*self.obj()).await;
        }

        #[template_callback]
        pub async fn on_continue_button(&self) {
            self.try_continue().handle_transient_for(&*self.obj()).await;
        }

        #[template_callback]
        fn validate(&self) {
            let kind = self.location_kind.get();
            let valid = self.selected_location().is_ok();

            // Folder row
            if !valid
                && kind == SetupLocationKind::Local
                && self.location_folder_row.file().is_none()
                && self.location_local_group_focus.contains_focus()
                && !self.location_folder_row_focus.contains_focus()
            {
                self.location_folder_row.add_css_class("error");
                self.location_folder_row
                    .update_state(&[gtk::accessible::State::Invalid(
                        gtk::AccessibleInvalidState::True,
                    )]);
            } else {
                self.location_folder_row.remove_css_class("error");
                self.location_folder_row
                    .reset_state(gtk::AccessibleState::Invalid);
            }

            // Repo name
            if !valid && kind == SetupLocationKind::Local && self.init_dir.text().is_empty() {
                self.init_dir.add_css_class("error");
                self.init_dir.delegate().inspect(|d| {
                    d.update_state(&[gtk::accessible::State::Invalid(
                        gtk::AccessibleInvalidState::True,
                    )]);
                });
            } else {
                self.init_dir.remove_css_class("error");
                self.init_dir.delegate().inspect(|d| {
                    d.reset_state(gtk::AccessibleState::Invalid);
                });
            }

            // Remote URL: If empty, only add error when not focused, to have a clean
            // initial state
            if !valid
                && kind == SetupLocationKind::Remote
                && (!self.location_url.text().is_empty()
                    || (self.location_remote_group_focus.contains_focus()
                        && !self.location_url_focus.contains_focus()))
            {
                self.location_url.add_css_class("error");
                self.location_url.delegate().inspect(|d| {
                    d.update_state(&[gtk::accessible::State::Invalid(
                        gtk::AccessibleInvalidState::True,
                    )])
                });
            } else {
                self.location_url.remove_css_class("error");
                self.location_url.delegate().inspect(|d| {
                    d.reset_state(gtk::AccessibleState::Invalid);
                });
            }

            self.obj().notify_can_continue();
        }

        fn selected_location(&self) -> Result<SetupRepoLocation> {
            // This might be called during template initialisation, make sure this uses
            // try_get everywhere
            match self.location_kind.get() {
                SetupLocationKind::Local => {
                    // We can only be here because we are creating a new repository
                    // If we were adding an existing one, this page would have been skipped

                    // Repo dir must not be empty
                    let repo_dir = self
                        .init_dir
                        .try_get()
                        .map(|d| d.text())
                        .unwrap_or_default();

                    if repo_dir.is_empty() {
                        return Err(Message::new(
                            gettext("Repository Name Empty"),
                            gettext("The repository folder name must be set"),
                        )
                        .into());
                    }

                    Ok(SetupRepoLocation::from_file(
                        self.location_folder_row
                            .try_get()
                            .and_then(|r| r.file())
                            .map(|p| p.child(repo_dir))
                            .ok_or_else(|| {
                                Message::new(
                                    gettext("No Base Folder Selected"),
                                    gettext("Select a base folder for the backup repository"),
                                )
                            })?,
                    ))
                }
                SetupLocationKind::Remote => SetupRepoLocation::parse_url(
                    self.location_url
                        .try_get()
                        .map(|u| u.text().to_string())
                        .unwrap_or_default(),
                )
                .err_to_msg(gettext("Invalid Remote Location")),
            }
        }
    }
}

glib::wrapper! {
    pub struct SetupLocationPage(ObjectSubclass<imp::SetupLocationPage>)
    @extends DialogPage, adw::NavigationPage, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SetupLocationPage {
    pub fn configure(
        &self,
        action: SetupAction,
        repo_kind: SetupLocationKind,
        file: Option<gio::File>,
    ) {
        self.imp().reset();
        self.set_action(action);
        self.set_location_kind(repo_kind);

        self.imp().location_folder_row.set_file(file);
    }
}
