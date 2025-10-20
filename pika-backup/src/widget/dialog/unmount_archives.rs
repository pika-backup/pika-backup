use std::collections::BTreeMap;
use std::time::Duration;

use adw::prelude::*;
use adw::subclass::prelude::*;
use adw::{glib, gtk};
use common::borg;
use common::config::ConfigId;
use common::utils::LookupConfigId;

use crate::error::{Error, Result};
use crate::prelude::ArcSwapResultExt;
use crate::{BACKUP_CONFIG, BACKUP_HISTORY};

mod imp {
    use super::*;
    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "unmount_archives.ui")]
    pub struct UnmountArchives {
        #[template_child]
        pub(super) error_info: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) error_list: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for UnmountArchives {
        const NAME: &'static str = "PkUnmountArchives";
        type Type = super::UnmountArchives;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for UnmountArchives {}
    impl WidgetImpl for UnmountArchives {}
    impl WindowImpl for UnmountArchives {}
    impl AdwWindowImpl for UnmountArchives {}
}

glib::wrapper! {
    pub struct UnmountArchives(ObjectSubclass<imp::UnmountArchives>)
    @extends gtk::Widget, gtk::Window, adw::Window,
    @implements gtk::Buildable, gtk::ConstraintTarget, gtk::Accessible, gtk::ShortcutManager, gtk::Root, gtk::Native;
}

impl UnmountArchives {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn add_row(&self, title: &str, subtitle: &str) -> (adw::ActionRow, gtk::Label) {
        let row = adw::ActionRow::builder()
            .title(title)
            .subtitle(subtitle)
            .build();

        let label = gtk::Label::builder().wrap(true).build();
        let popover = gtk::Popover::builder().child(&label).build();
        let menu_button = gtk::MenuButton::builder()
            .icon_name("info-outline")
            .valign(gtk::Align::Center)
            .popover(&popover)
            .build();

        row.add_suffix(&menu_button);

        self.imp().error_list.append(&row);
        self.imp().error_info.set_visible(true);

        (row, label)
    }

    pub fn remove_row(&self, widget: &adw::ActionRow) {
        self.imp().error_list.remove(widget);
    }

    pub async fn execute(&self, parent: &impl IsA<gtk::Window>) -> Result<()> {
        self.set_modal(true);
        self.set_transient_for(Some(parent));
        self.set_visible(true);

        let mut unmount_errors: BTreeMap<ConfigId, (adw::ActionRow, gtk::Label)> = BTreeMap::new();

        while BACKUP_HISTORY.load().iter().any(|(_, x)| x.is_browsing()) {
            for (config_id, _) in BACKUP_HISTORY
                .load()
                .iter()
                .filter(|(_, x)| x.is_browsing())
            {
                match BACKUP_CONFIG.load().try_get(config_id) {
                    Err(err) => {
                        tracing::error!("Can't unmount: {err:?}");
                    }
                    Ok(config) => match borg::functions::umount(&config.repo_id).await {
                        Err(err) => {
                            if let Some((_, label)) = unmount_errors.get(config_id) {
                                label.set_label(&err.to_string());
                            } else {
                                let widget = self.add_row(&config.title(), &config.repo.location());
                                unmount_errors.insert(config_id.clone(), widget);
                            }
                        }
                        Ok(()) => {
                            let _ = BACKUP_HISTORY
                                .try_update(|histories| {
                                    histories.remove_browsing(config_id.clone());
                                    Ok(())
                                })
                                .await;
                            let widget = unmount_errors.remove(config_id);
                            if let Some((widget, _)) = widget {
                                self.remove_row(&widget);
                            }
                        }
                    },
                }
            }

            if !self.is_mapped() {
                tracing::debug!("Dialog closed. Stopping unmounts.");
                return Err(Error::UserCanceled);
            }

            smol::Timer::after(Duration::from_millis(300)).await;
        }

        Ok(())
    }
}
