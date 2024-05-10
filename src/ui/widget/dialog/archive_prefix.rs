use adw::prelude::*;
use ui::prelude::*;

use crate::config;
use crate::ui;

use adw::subclass::prelude::*;

mod imp {
    use super::*;
    use std::cell::OnceCell;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::ArchivePrefixDialog)]
    #[template(file = "archive_prefix.ui")]
    pub struct ArchivePrefixDialog {
        #[property(get, set, construct_only)]
        pub config_id: OnceCell<ConfigId>,

        #[template_child]
        archive_prefix: TemplateChild<adw::EntryRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ArchivePrefixDialog {
        const NAME: &'static str = "PkArchivePrefixDialog";
        type Type = super::ArchivePrefixDialog;
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
    impl ObjectImpl for ArchivePrefixDialog {
        fn constructed(&self) {
            self.parent_constructed();

            let Ok(config) = self.config() else {
                return;
            };

            self.archive_prefix
                .set_text(&config.archive_prefix.to_string());
            self.archive_prefix.grab_focus();
        }
    }

    impl WidgetImpl for ArchivePrefixDialog {}
    impl WindowImpl for ArchivePrefixDialog {}
    impl AdwWindowImpl for ArchivePrefixDialog {}

    #[gtk::template_callbacks]
    impl ArchivePrefixDialog {
        fn config(&self) -> Result<crate::config::Backup> {
            match BACKUP_CONFIG.load().try_get(self.config_id.get().unwrap()) {
                Ok(backup) => Ok(backup.clone()),
                Err(err) => Err(crate::ui::Error::from(err)),
            }
        }

        async fn save(&self) -> Result<()> {
            let new_prefix = self.archive_prefix.text();

            let mut config = self.config()?;
            if config.prune.enabled {
                config
                    .set_archive_prefix(
                        config::ArchivePrefix::new(&new_prefix),
                        BACKUP_CONFIG.load().iter(),
                    )
                    .err_to_msg(gettext("Invalid Archive Prefix"))?;

                self.obj().close();
                ui::widget::dialog::PruneReviewDialog::review(
                    &ui::App::default().main_window(),
                    &config,
                )
                .await?;
            }

            let imp = self.ref_counted();
            BACKUP_CONFIG
                .try_update(enclose!(
                    (imp) | config | {
                        config
                            .try_get_mut(imp.config_id.get().unwrap())?
                            .set_archive_prefix(
                                config::ArchivePrefix::new(&new_prefix),
                                BACKUP_CONFIG.load().iter(),
                            )
                            .err_to_msg(gettext("Invalid Archive Prefix"))?;
                        Ok(())
                    }
                ))
                .await?;

            main_ui()
                .page_detail()
                .archives_page()
                .update_info(BACKUP_CONFIG.load().active()?);

            self.obj().close();

            Ok(())
        }

        #[template_callback]
        async fn on_save(&self) {
            let obj = self.obj().clone();
            Handler::new()
                .error_transient_for(obj.clone())
                .spawn(async move { obj.imp().save().await });
        }
    }
}

glib::wrapper! {
    pub struct ArchivePrefixDialog(ObjectSubclass<imp::ArchivePrefixDialog>)
    @extends gtk::Widget, gtk::Window, adw::Window,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ArchivePrefixDialog {
    pub fn new(config_id: ConfigId) -> Self {
        glib::Object::builder()
            .property("config-id", config_id)
            .build()
    }
}
