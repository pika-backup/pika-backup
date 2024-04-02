pub mod add_task;
mod display;
mod event;
pub mod folder_button;
mod insert;
mod remote_location;
mod start;
mod transfer_option;

use adw::prelude::*;
use adw::subclass::prelude::*;

use start::SetupStartPage;

use crate::ui::prelude::*;
use crate::ui::widget::EncryptionPreferencesGroup;
use add_task::AddConfigTask;
use folder_button::FolderButton;

#[derive(Debug, Copy, Clone, PartialEq, Eq, glib::Enum, Default)]
#[enum_type(name = "PkSetupKind")]
pub enum SetupKind {
    #[default]
    Init,
    AddExisting,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, glib::Enum, Default)]
#[enum_type(name = "PkSetupRepoKind")]
pub enum SetupRepoKind {
    #[default]
    Local,
    Remote,
}

mod imp {

    use crate::{
        config,
        ui::{self, error::HandleError},
    };

    use super::*;
    use std::cell::Cell;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[template(file = "setup.ui")]
    #[properties(wrapper_type = super::SetupDialog)]
    pub struct SetupDialog {
        #[property(get, set)]
        prop: Cell<bool>,

        #[template_child]
        pub(super) navigation_view: TemplateChild<adw::NavigationView>,

        // Initial screen
        #[template_child]
        pub(super) start_page: TemplateChild<SetupStartPage>,

        // First page
        #[template_child]
        pub(super) page_detail: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub(super) location_group_local: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) show_settings: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) location_local: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) init_path: TemplateChild<FolderButton>,
        #[template_child]
        pub(super) init_dir: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) non_journaling_warning: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) location_group_remote: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) location_url: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) command_line_args_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) button_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) add_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) page_detail_continue: TemplateChild<gtk::Button>,

        // Encryption page
        #[template_child]
        pub(super) page_setup_encryption: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub(super) encryption_preferences_group: TemplateChild<EncryptionPreferencesGroup>,
        #[template_child]
        pub(super) init_button: TemplateChild<gtk::Button>,

        // Ask for password page
        #[template_child]
        pub(super) page_password: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub(super) page_password_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) page_password_pending: TemplateChild<gtk::WindowHandle>,
        #[template_child]
        pub(super) pending_spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub(super) page_password_input: TemplateChild<adw::ToolbarView>,
        #[template_child]
        pub(super) ask_password: TemplateChild<gtk::PasswordEntry>,
        #[template_child]
        pub(super) page_password_continue: TemplateChild<gtk::Button>,

        // Transfer settings page
        #[template_child]
        pub(super) page_transfer: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub(super) page_transfer_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) page_transfer_pending: TemplateChild<adw::ToolbarView>,
        #[template_child]
        pub(super) transfer_pending_spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub(super) page_transfer_select: TemplateChild<adw::ToolbarView>,
        #[template_child]
        pub(super) transfer_suggestions: TemplateChild<gtk::ListBox>,

        // Transfer prefix page
        #[template_child]
        pub(super) page_transfer_prefix: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub(super) prefix: TemplateChild<gtk::Entry>,
        #[template_child]
        pub(super) prefix_submit: TemplateChild<gtk::Button>,

        // Creating spinner page
        #[template_child]
        pub(super) page_creating: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub(super) creating_repository_spinner: TemplateChild<gtk::Spinner>,

        #[template_child]
        pub(super) add_task: TemplateChild<AddConfigTask>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SetupDialog {
        const NAME: &'static str = "PkSetupDialog";
        type Type = super::SetupDialog;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for SetupDialog {
        fn constructed(&self) {
            self.parent_constructed();
            let dialog = self.obj();

            // Default buttons

            self.page_detail_continue
                .connect_map(clone!(@weak dialog => move |x| dialog.set_default_widget(Some(x))));
            self.init_button
                .connect_map(clone!(@weak dialog => move |x| dialog.set_default_widget(Some(x))));
            self.add_button
                .connect_map(clone!(@weak dialog => move |x| dialog.set_default_widget(Some(x))));
            self.prefix_submit
                .connect_map(clone!(@weak dialog => move |x| dialog.set_default_widget(Some(x))));
            self.page_password_continue
                .connect_map(clone!(@weak dialog => move |x| dialog.set_default_widget(Some(x))));

            // Page Overview

            let imp = self.ref_counted();

            // Page Detail

            self.navigation_view.connect_visible_page_notify(
                clone!(@weak imp => move |_| imp.event_navigation_view_changed()),
            );

            self.page_detail_continue
                .connect_clicked(clone!(@weak imp => move |_| imp.event_page_detail_continue()));

            self.init_path
                .connect_folder_change(clone!(@weak imp => move || imp.event_path_change()));

            // Page Setup Encryption
            self.add_button
                .connect_clicked(clone!(@weak imp => move |_| {
                    let obj = imp.obj().clone();
                    Self::execute(async move { imp.event_add_remote().await }, obj)
                }));

            self.init_button
                .connect_clicked(clone!(@weak imp => move |_| imp.event_init_repo()));

            // Page Password

            let run = clone!(@weak imp => move |x| {
                let obj = imp.obj().clone();
                Self::execute(x, obj)
            });

            self.page_password_continue.connect_clicked(
                clone!(@weak imp => move |_| run(async move { imp.event_page_password_continue().await })),
            );

            self.page_password_stack.connect_visible_child_notify(
                clone!(@weak imp => move |_| imp.event_navigation_view_changed()),
            );

            self.pending_spinner.connect_map(|s| s.start());
            self.pending_spinner.connect_unmap(|s| s.stop());

            self.transfer_pending_spinner.connect_map(|s| s.start());
            self.transfer_pending_spinner.connect_unmap(|s| s.stop());

            self.creating_repository_spinner.connect_map(|s| s.start());
            self.creating_repository_spinner.connect_unmap(|s| s.stop());

            let start_page = self.start_page.clone();
            glib::MainContext::default().spawn_local(async move {
                Handler::handle(start_page.refresh().await);
            });
        }
    }
    impl WidgetImpl for SetupDialog {}
    impl WindowImpl for SetupDialog {}
    impl AdwWindowImpl for SetupDialog {}

    #[gtk::template_callbacks]
    impl SetupDialog {
        async fn show_add_existing_file_chooser(&self) -> Result<Option<gio::File>> {
            if let Some(path) =
                ui::utils::folder_chooser_dialog(&gettext("Setup Existing Repository"), None)
                    .await
                    .ok()
                    .and_then(|x| x.path())
            {
                self.obj().set_visible(true);
                if ui::utils::is_backup_repo(&path).await {
                    return Ok(Some(gio::File::for_path(path)));
                } else {
                    return Err(Message::new(
                        gettext("Location is not a valid backup repository."),
                        gettext(
                            "The repository must originate from Pika Backup or compatible software.",
                        ),
                    )
                    .into());
                }
            }

            Ok(None)
        }

        #[template_callback]
        async fn on_start_page_continue(
            &self,
            kind: SetupKind,
            repo: SetupRepoKind,
            file: Option<gio::File>,
        ) {
            match (kind, repo, file) {
                (SetupKind::Init, SetupRepoKind::Local, file) => {
                    self.show_init_local(file.and_then(|f| f.path()).as_deref());
                }
                (SetupKind::Init, SetupRepoKind::Remote, _) => self.show_init_remote(),
                (SetupKind::AddExisting, SetupRepoKind::Local, file) => {
                    let path = if let Some(file) = file {
                        file.path()
                    } else {
                        self.show_add_existing_file_chooser()
                            .await
                            .handle_transient_for(&*self.obj())
                            .await
                            .flatten()
                            .and_then(|f| f.path())
                    };

                    let Some(path) = path else {
                        return;
                    };

                    let result = self
                        .add_first_try(config::local::Repository::from_path(path).into_config())
                        .await;
                    // add_first_try moves us to detail, fix here for now
                    if !matches!(result, Err(Error::UserCanceled) | Ok(())) {
                        result.handle_transient_for(&*self.obj()).await;
                        self.navigation_view.pop_to_page(&*self.start_page);
                    }
                }
                (SetupKind::AddExisting, SetupRepoKind::Remote, _) => {
                    self.event_show_add_remote();
                }
            }
        }
    }
}

glib::wrapper! {
    pub struct SetupDialog(ObjectSubclass<imp::SetupDialog>)
    @extends gtk::Widget, gtk::Window, adw::Window,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SetupDialog {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn present_with(&self, transient_for: &impl IsA<gtk::Window>) {
        self.set_transient_for(Some(transient_for));
        self.present();
    }
}
