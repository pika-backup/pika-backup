use gtk::prelude::*;

use crate::config;
use crate::ui;

use adw::prelude::*;
use adw::subclass::prelude::*;

mod imp {
    use super::*;
    use std::cell::OnceCell;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[template(file = "storage.ui")]
    #[properties(wrapper_type = super::StorageDialog)]
    pub struct StorageDialog {
        #[property(get, set, construct_only)]
        config: OnceCell<crate::config::Backup>,

        #[template_child]
        disk_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        volume_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        device_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        path_row: TemplateChild<adw::ActionRow>,

        #[template_child]
        remote_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        uri_row: TemplateChild<adw::ActionRow>,

        #[template_child]
        fs_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        fs_size_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        fs_free_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        fs_usage_bar: TemplateChild<gtk::LevelBar>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StorageDialog {
        const NAME: &'static str = "PkStorageDialog";
        type Type = super::StorageDialog;
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
    impl ObjectImpl for StorageDialog {
        fn constructed(&self) {
            self.parent_constructed();

            let config = self
                .config
                .get()
                .expect("construct_only property must be set");

            match &config.repo {
                config::Repository::Local(repo) => {
                    self.volume_row
                        .set_subtitle(&repo.mount_name.clone().unwrap_or_default());
                    self.device_row
                        .set_subtitle(&repo.drive_name.clone().unwrap_or_default());
                    self.path_row.set_subtitle(&repo.path().to_string_lossy());
                    self.disk_group.set_visible(true);
                }
                config::Repository::Remote { .. } => {
                    self.uri_row.set_subtitle(&config.repo.to_string());

                    self.remote_group.set_visible(true);
                }
            }
        }
    }
    impl WidgetImpl for StorageDialog {}
    impl AdwDialogImpl for StorageDialog {}

    #[gtk::template_callbacks]
    impl StorageDialog {
        pub(super) fn set_df(&self, df: &config::Space) {
            self.fs_size_row.set_subtitle(&glib::format_size(df.size));
            self.fs_free_row.set_subtitle(&glib::format_size(df.avail));
            self.fs_usage_bar
                .set_value(1.0 - df.avail as f64 / df.size as f64);
            self.fs_group.set_visible(true);
        }

        pub(super) async fn refresh(&self) {
            if let Some(df) = ui::utils::df::cached_or_lookup(self.config.get().unwrap()).await {
                self.set_df(&df);
            }
        }
    }
}

glib::wrapper! {
    pub struct StorageDialog(ObjectSubclass<imp::StorageDialog>)
    @extends gtk::Widget, adw::Dialog,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl StorageDialog {
    pub async fn new(config: &config::Backup) -> Self {
        let obj: Self = glib::Object::builder().property("config", config).build();
        obj.refresh().await;
        obj
    }

    pub async fn refresh(&self) {
        self.imp().refresh().await;
    }
}
