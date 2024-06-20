use std::fmt::Write;

use adw::prelude::*;
use adw::subclass::prelude::*;
use async_std::stream::StreamExt;

use super::{SetupAction, SetupLocationKind};
use crate::ui;
use crate::ui::prelude::*;

const LISTED_URI_SCHEMES: &[&str] = &["file", "smb", "sftp", "ssh"];

mod imp {
    use adw::subclass::navigation_page::NavigationPageImplExt;
    use glib::subclass::Signal;

    use self::ui::widget::dialog_page::PkDialogPageImpl;

    use super::*;
    use std::{cell::Cell, sync::OnceLock};

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[template(file = "location_kind.ui")]
    #[properties(wrapper_type = super::SetupLocationKindPage)]
    pub struct SetupLocationKindPage {
        #[property(get, set = Self::set_repo_action, builder(SetupAction::Init))]
        repo_action: Cell<SetupAction>,

        #[template_child]
        page: TemplateChild<adw::PreferencesPage>,
        #[template_child]
        create_new_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        init_repo_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        init_local_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        init_remote_row: TemplateChild<adw::ActionRow>,

        #[template_child]
        add_existing_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        add_repo_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        add_local_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        add_remote_row: TemplateChild<adw::ActionRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SetupLocationKindPage {
        const NAME: &'static str = "PkSetupLocationKindPage";
        type Type = super::SetupLocationKindPage;
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
    impl ObjectImpl for SetupLocationKindPage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![Signal::builder("continue")
                    .param_types([
                        SetupAction::static_type(),
                        SetupLocationKind::static_type(),
                        Option::<gio::File>::static_type(),
                    ])
                    .build()]
            })
        }

        fn constructed(&self) {
            self.parent_constructed();

            let volume_monitor = gio::VolumeMonitor::get();
            let imp = self.ref_counted();

            volume_monitor.connect_mount_added(clone!(
                #[weak]
                imp,
                move |_, mount| {
                    debug!("Mount added");
                    let mount = mount.clone();
                    if let Some(window) = imp.obj().root().and_downcast_ref::<gtk::Window>() {
                        Handler::new()
                            .error_transient_for(window.clone())
                            .spawn(async move { imp.load_mount(mount.clone()).await });
                    }
                }
            ));

            volume_monitor.connect_mount_removed(clone!(
                #[weak]
                imp,
                move |_, mount| {
                    debug!("Mount removed");
                    Self::remove_mount(&imp.add_repo_list, &mount.root().uri());
                    Self::remove_mount(&imp.init_repo_list, &mount.root().uri());
                }
            ));
        }
    }
    impl WidgetImpl for SetupLocationKindPage {}
    impl NavigationPageImpl for SetupLocationKindPage {
        fn shown(&self) {
            self.parent_shown();
            self.init_local_row.grab_focus();
        }
    }
    impl PkDialogPageImpl for SetupLocationKindPage {}

    #[gtk::template_callbacks]
    impl SetupLocationKindPage {
        fn emit_continue(
            &self,
            kind: SetupAction,
            repo_kind: SetupLocationKind,
            file: Option<&gio::File>,
        ) {
            self.obj()
                .emit_by_name::<()>("continue", &[&kind, &repo_kind, &file]);
        }

        fn set_repo_action(&self, action: SetupAction) {
            self.create_new_group
                .set_visible(action == SetupAction::Init);
            self.add_existing_group
                .set_visible(action == SetupAction::AddExisting);

            match action {
                SetupAction::Init => {
                    self.obj().set_title(&gettext("Create new Repository"));
                    self.page.set_description(&gettext(
                        "Select a location for the new backup repository",
                    ));
                }
                SetupAction::AddExisting => {
                    self.obj().set_title(&gettext("Use Existing Repository"));
                    self.page.set_description(&gettext(
                        "Select a location that contains an existing backup repository. Repositories created with other BorgBackup compatible software can be used as well."
                    ));
                }
            }

            self.repo_action.set(action);
        }

        #[template_callback]
        fn on_row_activated(&self, row: &adw::ActionRow) {
            let (kind, repo) = if row == &*self.init_local_row {
                (SetupAction::Init, SetupLocationKind::Local)
            } else if row == &*self.init_remote_row {
                (SetupAction::Init, SetupLocationKind::Remote)
            } else if row == &*self.add_local_row {
                (SetupAction::AddExisting, SetupLocationKind::Local)
            } else if row == &*self.add_remote_row {
                (SetupAction::AddExisting, SetupLocationKind::Remote)
            } else {
                // Unreachable
                return;
            };

            self.emit_continue(kind, repo, None);
        }

        pub(super) fn clear(&self) {
            ui::utils::clear(&self.add_repo_list);
            ui::utils::clear(&self.init_repo_list);
        }

        pub(super) async fn load_mount(&self, mount: gio::Mount) -> Result<()> {
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
                    clone!(
                        #[weak]
                        imp,
                        #[strong]
                        mount_point,
                        move || {
                            imp.emit_continue(
                                SetupAction::Init,
                                SetupLocationKind::Local,
                                Some(&gio::File::for_path(&mount_point)),
                            );
                        }
                    ),
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
                        glib::clone!(
                            #[weak]
                            imp,
                            #[strong]
                            path,
                            move || {
                                imp.emit_continue(
                                    SetupAction::AddExisting,
                                    SetupLocationKind::Local,
                                    Some(&gio::File::for_path(&path)),
                                );
                            }
                        ),
                    )
                    .await;
                }
            }

            Ok(())
        }

        pub(super) fn remove_mount(list: &gtk::ListBox, root: &str) {
            let mut i = 0;
            while let Some(list_row) = list.row_at_index(i) {
                if list_row.widget_name().starts_with(root) {
                    list.remove(&list_row);
                    break;
                }
                i += 1
            }
        }

        pub async fn add_mount<F: 'static + Fn()>(
            list: &gtk::ListBox,
            mount: &gio::Mount,
            repo: Option<&std::path::Path>,
            display_fn: F,
        ) {
            let row = ui::utils::new_action_row_with_gicon(Some(mount.icon().as_ref()));
            list.append(&row);

            row.set_widget_name(&mount.root().uri());
            row.connect_activated(move |_| display_fn());
            row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));

            let mut label1 = mount.name().to_string();

            let mut label2: String = mount
                .drive()
                .as_ref()
                .map(gio::Drive::name)
                .map(Into::into)
                .unwrap_or_else(|| mount.root().uri().to_string());

            if let Some(mount_path) = mount.root().path() {
                if let Ok(df) = ui::utils::df::local(&mount_path).await {
                    let _ = write!(label1, " – {}", &glib::format_size(df.size));

                    label2.push_str(" – ");
                    label2.push_str(&gettextf("Free space: {}", &[&glib::format_size(df.avail)]));
                }

                if let Some(repo_path) = repo {
                    row.set_widget_name(&gio::File::for_path(repo_path).uri());
                    if let Ok(suffix) = repo_path.strip_prefix(mount_path) {
                        if !suffix.to_string_lossy().is_empty() {
                            let _ = write!(label1, " / {}", suffix.display());
                        }
                    }
                }
            }

            row.set_title(&glib::markup_escape_text(&label1));
            row.set_subtitle(&glib::markup_escape_text(&label2));
        }

        pub(super) async fn refresh(&self) -> Result<()> {
            debug!("Refreshing list of existing repos");
            let monitor = gio::VolumeMonitor::get();

            self.clear();

            for mount in monitor.mounts() {
                self.load_mount(mount).await?;
            }

            debug!("List of existing repos refreshed");
            Ok(())
        }
    }
}

glib::wrapper! {
    pub struct SetupLocationKindPage(ObjectSubclass<imp::SetupLocationKindPage>)
    @extends DialogPage, adw::NavigationPage, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SetupLocationKindPage {
    pub async fn refresh(&self) -> Result<()> {
        self.imp().refresh().await
    }
}
