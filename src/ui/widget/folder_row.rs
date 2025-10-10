use crate::ui::prelude::*;
use adw::prelude::*;
use adw::subclass::prelude::*;

mod imp {
    use super::*;

    use glib::Properties;
    use gtk::CompositeTemplate;
    use std::cell::RefCell;

    #[derive(Default, CompositeTemplate, Properties)]
    #[template(file = "folder_row.ui")]
    #[properties(wrapper_type = super::FolderRow)]
    pub struct FolderRow {
        #[property(get, set = Self::set_file, nullable)]
        file: RefCell<Option<gio::File>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FolderRow {
        const NAME: &'static str = "PkFolderRow";
        type Type = super::FolderRow;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FolderRow {}

    impl WidgetImpl for FolderRow {}
    impl ListBoxRowImpl for FolderRow {}
    impl PreferencesRowImpl for FolderRow {}
    impl ActionRowImpl for FolderRow {
        fn activate(&self) {
            Handler::run(glib::clone!(
                #[strong(rename_to = obj)]
                self.obj(),
                async move {
                    let preselect = match obj.file() {
                        Some(file) => file,
                        _ => gio::File::for_path(glib::home_dir()),
                    };

                    let file = crate::ui::utils::folder_chooser_dialog(
                        &gettext("Backup Location"),
                        Some(&preselect),
                    )
                    .await?;

                    obj.set_file(Some(file));

                    Ok(())
                }
            ));
        }
    }

    impl FolderRow {
        fn set_file(&self, file: Option<gio::File>) {
            if let Some(file) = &file {
                self.obj().add_css_class("property");
                self.obj().set_subtitle(
                    &file
                        .path()
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .map(|o| o.to_string_lossy())
                        .unwrap_or_default(),
                );
            } else {
                self.obj().set_subtitle("");
                self.obj().remove_css_class("property");
            }

            self.file.replace(file);
        }
    }
}

glib::wrapper! {
    pub struct FolderRow(ObjectSubclass<imp::FolderRow>)
        @extends adw::ActionRow, adw::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Actionable;
}

impl FolderRow {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn reset(&self) {
        self.set_file(None::<gio::File>);
    }
}
