use crate::ui::prelude::*;
use adw::prelude::*;
use adw::subclass::prelude::*;

use super::types::*;
use super::SetupRepoLocation;
use crate::ui::widget::DialogPage;

mod imp {
    use std::{cell::Cell, sync::OnceLock};

    use gettextrs::gettext;
    use glib::{subclass::Signal, WeakRef};

    use crate::ui::{
        error::HandleError,
        widget::{
            folder_row::FolderRow, setup::advanced_options::SetupAdvancedOptionsPage,
            PkDialogPageImpl,
        },
    };

    use super::*;

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

        #[template_child]
        advanced_options_page: TemplateChild<SetupAdvancedOptionsPage>,
        #[template_child]
        pub(super) location_group_local: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) location_folder_row: TemplateChild<FolderRow>,
        #[template_child]
        pub(super) init_dir: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) non_journaling_warning: TemplateChild<gtk::Box>,
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
                vec![Signal::builder("continue")
                    .param_types([
                        SetupRepoLocation::static_type(),
                        SetupCommandLineArgs::static_type(),
                    ])
                    .build()]
            })
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
        fn on_folder_changed(&self) {
            let folder = self.location_folder_row.file();

            if folder.is_some() {
                self.location_folder_row
                    .set_title(&gettext("Repository Folder"));
            } else {
                self.location_folder_row
                    .set_title(&gettext("Choose Repository Folder"));
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
        }

        fn try_continue(&self) -> Result<()> {
            let repo_location = self.selected_location()?;
            let command_line_args = self.advanced_options_page.selected_command_line_args()?;

            debug!("Continue with repo location '{}'", repo_location);

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
        fn on_path_change(&self) {
            if let Some(path) = self.location_folder_row.file().and_then(|x| x.path()) {
                let mount_entry = gio::UnixMountEntry::for_file_path(path);
                if let Some(fs) = mount_entry.0.map(|x| x.fs_type()) {
                    debug!("Selected filesystem type {}", fs);
                    self.non_journaling_warning
                        .set_visible(crate::NON_JOURNALING_FILESYSTEMS.iter().any(|x| x == &fs));
                } else {
                    self.non_journaling_warning.set_visible(false);
                }
            } else {
                self.non_journaling_warning.set_visible(false);
            }
        }

        fn selected_location(&self) -> Result<SetupRepoLocation> {
            match self.location_kind.get() {
                SetupLocationKind::Local => {
                    // We can only be here because we are creating a new repository
                    // If we were adding an existing one, this page would have been skipped

                    // Repo dir must not be empty
                    let repo_dir = self.init_dir.text();
                    if repo_dir.is_empty() {
                        return Err(Message::new(
                            gettext("Repository Name Empty"),
                            gettext("The repository folder name must be set"),
                        )
                        .into());
                    }

                    Ok(SetupRepoLocation::from_file(
                        self.location_folder_row
                            .file()
                            .map(|p| p.child(repo_dir))
                            .ok_or_else(|| {
                                Message::new(
                                    gettext("No Base Folder Selected"),
                                    gettext("Select a base folder for the backup repository"),
                                )
                            })?,
                    ))
                }
                SetupLocationKind::Remote => {
                    SetupRepoLocation::parse_url(self.location_url.text().to_string())
                        .err_to_msg(gettext("Invalid Remote Location"))
                }
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
