pub mod add_task;
mod display;
mod event;
pub mod folder_button;
mod insert;
mod remote_location;
mod transfer_option;

use adw::prelude::*;
use adw::subclass::prelude::*;
use async_std::stream::StreamExt;

use crate::ui;
use crate::ui::prelude::*;
use crate::ui::widget::EncryptionPreferencesGroup;
use add_task::AddConfigTask;
use folder_button::FolderButton;

const LISTED_URI_SCHEMES: &[&str] = &["file", "smb", "sftp", "ssh"];

mod imp {
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
        pub(super) page_overview: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub(super) init_repo_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) init_local_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) init_remote_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) add_repo_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) add_local_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) add_remote_row: TemplateChild<adw::ActionRow>,

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
            self.init_local_row
                .connect_activated(clone!(@weak imp => move |_| imp.event_show_init_local()));

            self.init_remote_row
                .connect_activated(clone!(@weak imp => move |_| imp.event_show_init_remote()));

            self.add_local_row
                .connect_activated(clone!(@weak imp =>  move |_| imp.event_show_add_local()));

            self.add_remote_row
                .connect_activated(clone!(@weak imp => move |_| imp.event_show_add_remote()));

            self.load_available_mounts_and_repos();

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

            // refresh ui on mount events

            let volume_monitor = gio::VolumeMonitor::get();

            volume_monitor.connect_mount_added(clone!(@weak dialog => move |_, mount| {
                debug!("Mount added");
                let mount = mount.clone();
                Handler::new().error_transient_for(dialog.clone())
                .spawn(async move { dialog.imp().load_mount(mount.clone()).await });
            }));

            volume_monitor.connect_mount_removed(clone!(@weak dialog => move |_, mount| {
                debug!("Mount removed");
                Self::remove_mount(&dialog.imp().add_repo_list, &mount.root().uri());
                Self::remove_mount(
                    &dialog.imp().init_repo_list,
                    &mount.root().uri(),
                );
            }));
        }
    }
    impl WidgetImpl for SetupDialog {}
    impl WindowImpl for SetupDialog {}
    impl AdwWindowImpl for SetupDialog {}

    #[gtk::template_callbacks]
    impl SetupDialog {
        fn load_available_mounts_and_repos(&self) {
            debug!("Refreshing list of existing repos");
            let monitor = gio::VolumeMonitor::get();

            ui::utils::clear(&self.add_repo_list);
            ui::utils::clear(&self.init_repo_list);

            for mount in monitor.mounts() {
                let obj = self.obj().clone();
                Handler::new()
                    .error_transient_for(self.obj().clone())
                    .spawn(async move { obj.imp().load_mount(mount).await });
            }

            debug!("List of existing repos refreshed");
        }

        async fn load_mount(&self, mount: gio::Mount) -> Result<()> {
            let uri_scheme = mount
                .root()
                .uri_scheme()
                .unwrap_or_else(|| glib::GString::from(""))
                .to_lowercase();

            if !LISTED_URI_SCHEMES.contains(&uri_scheme.as_str()) {
                info!("Ignoring volume because of URI scheme '{}'", uri_scheme);
                return Ok(());
            }

            let imp = self.ref_counted();

            if let Some(mount_point) = mount.root().path() {
                Self::add_mount(
                    &self.init_repo_list,
                    &mount,
                    None,
                    clone!(@weak imp, @strong mount_point => move || {
                        imp.show_init_local(Some(&mount_point))
                    }),
                )
                .await;

                let mut paths = Vec::new();
                if let Ok(mut dirs) = async_std::fs::read_dir(mount_point).await {
                    while let Some(Ok(path)) = dirs.next().await {
                        if ui::utils::is_backup_repo(path.path().as_ref()).await {
                            paths.push(path.path());
                        }
                    }
                }

                for path in paths {
                    trace!("Adding repo to ui '{:?}'", path);
                    Self::add_mount(
                        &self.add_repo_list,
                        &mount,
                        Some(path.as_ref()),
                        clone!(@weak imp, @strong path => move || {
                            imp.event_add_local(Some(path.as_ref()))
                        }),
                    )
                    .await;
                }
            }

            Ok(())
        }

        fn remove_mount(list: &gtk::ListBox, root: &str) {
            let mut i = 0;
            while let Some(list_row) = list.row_at_index(i) {
                if list_row.widget_name().starts_with(root) {
                    list.remove(&list_row);
                    break;
                }
                i += 1
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
