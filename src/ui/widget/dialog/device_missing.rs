use adw::prelude::*;
use adw::subclass::prelude::*;

use crate::config;
use crate::ui::prelude::*;

mod imp {
    use super::*;
    use std::cell::{OnceCell, RefCell};

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[template(file = "device_missing.ui")]
    #[properties(wrapper_type = super::DeviceMissingDialog)]
    pub struct DeviceMissingDialog {
        #[property(get, set, construct_only)]
        config: OnceCell<crate::config::Backup>,

        mount_sender: RefCell<Option<async_std::channel::Sender<Option<gio::Mount>>>>,

        #[template_child]
        name_label: TemplateChild<gtk::Label>,
        #[template_child]
        icon_box: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DeviceMissingDialog {
        const NAME: &'static str = "PkDeviceMissingDialog";
        type Type = super::DeviceMissingDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for DeviceMissingDialog {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }
    impl WidgetImpl for DeviceMissingDialog {}
    impl AdwDialogImpl for DeviceMissingDialog {
        fn closed(&self) {
            if let Some(sender) = self.mount_sender.take() {
                let _ = sender.try_send(None);
            }
        }
    }

    #[gtk::template_callbacks]
    impl DeviceMissingDialog {
        pub(super) fn monitor_repo(
            &self,
            repo: &config::local::Repository,
        ) -> async_std::channel::Receiver<Option<gio::Mount>> {
            self.name_label
                .set_label(&repo.clone().into_config().location());

            if let Some(g_icon) = repo
                .icon
                .as_ref()
                .and_then(|x| gio::Icon::for_string(x).ok())
            {
                let img = gtk::Image::from_gicon(&g_icon);
                img.set_pixel_size(128);
                self.icon_box.append(&img);
            }

            let dialog = self.obj();
            let volume_monitor = gio::VolumeMonitor::get();
            let (mount_sender, mount_receiver) = async_std::channel::unbounded();
            volume_monitor.connect_mount_added(
                enclose!((dialog, mount_sender, repo) move |_, new_mount| {
                    if let Some(volume) = new_mount.volume() {
                        if repo.is_likely_on_volume(&volume) {
                            let _ignore = mount_sender.try_send(Some(new_mount.clone()));
                            dialog.close();
                        } else {
                            debug!("New volume, but likely not on there.");
                        }
                    }
                }),
            );

            self.mount_sender.replace(Some(mount_sender));
            mount_receiver
        }
    }
}

glib::wrapper! {
    pub struct DeviceMissingDialog(ObjectSubclass<imp::DeviceMissingDialog>)
    @extends gtk::Widget, adw::Dialog,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl DeviceMissingDialog {
    pub fn new(config: &config::Backup) -> Self {
        glib::Object::builder().property("config", config).build()
    }

    pub async fn present_with_repo(
        &self,
        parent: &impl IsA<gtk::Widget>,
        repo: &config::local::Repository,
        purpose: &str,
    ) -> Result<gio::Mount> {
        self.set_title(purpose);

        let mount_receiver = self.imp().monitor_repo(repo);

        self.present(Some(parent));
        mount_receiver
            .recv()
            .await
            .ok()
            .flatten()
            .ok_or(Error::UserCanceled)
    }
}
