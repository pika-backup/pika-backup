use glib::subclass::prelude::*;
use gtk::glib;
use gtk::glib::prelude::*;

glib::wrapper! {
    pub struct FolderButton(ObjectSubclass<imp::FolderButton>)
        @extends gtk::Button, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl FolderButton {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn file(&self) -> Option<gio::File> {
        self.imp().file.borrow().clone()
    }

    pub fn connect_folder_change<F: 'static + Fn()>(&self, f: F) -> gtk::glib::SignalHandlerId {
        self.connect_notify_local(Some("file"), move |_, _| f())
    }

    pub fn reset(&self) {
        self.set_property("file", None::<gio::File>);
    }
}

mod imp {
    use crate::ui::prelude::*;
    use glib::{ParamSpec, ParamSpecObject, Value};
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use once_cell::sync::Lazy;
    use std::cell::RefCell;

    #[derive(Default)]
    pub struct FolderButton {
        pub file: RefCell<Option<gio::File>>,
        pub child: Lazy<gtk::Box>,
        pub image: Lazy<gtk::Image>,
        pub label: Lazy<gtk::Label>,
    }

    #[gtk::glib::object_subclass]
    impl ObjectSubclass for FolderButton {
        const NAME: &'static str = "PikaFolderButton";
        type Type = super::FolderButton;
        type ParentType = gtk::Button;
    }

    impl ObjectImpl for FolderButton {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> =
                Lazy::new(|| vec![ParamSpecObject::builder::<gio::File>("file").build()]);
            PROPERTIES.as_ref()
        }

        fn property(&self, __id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "file" => self.file.borrow().to_value(),
                _ => unimplemented!(),
            }
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "file" => {
                    let file = value.get::<gio::File>().ok();

                    if let Some(ref file) = file {
                        let info = file
                            .query_info(
                                "standard::*",
                                gtk::gio::FileQueryInfoFlags::NONE,
                                gtk::gio::Cancellable::NONE,
                            )
                            .ok();

                        let mount_icon = file
                            .find_enclosing_mount(gtk::gio::Cancellable::NONE)
                            .ok()
                            .map(|x| x.icon());
                        let file_icon = info.as_ref().and_then(|x| x.icon());

                        self.image
                            .set_gicon([mount_icon, file_icon].iter().flatten().next());
                        self.image.set_visible(true);

                        self.label.set_label(
                            &info
                                .map(|x| x.display_name().to_string())
                                .unwrap_or_default(),
                        );
                    } else {
                        self.image.set_visible(false);
                        self.reset_label();
                    }

                    self.file.replace(file);
                }
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            let obj = self.obj();
            self.parent_constructed();

            obj.set_child(Some(&*self.child));
            obj.add_css_class("folder-button");

            self.child.append(&*self.image);
            self.image.set_visible(false);

            self.child.append(&*self.label);
            self.reset_label();
            self.label.set_mnemonic_widget(Some(&*obj));

            obj.connect_clicked(|obj| {
                let obj = obj.clone();
                Handler::run(async move {
                    let preselect = if let Some(file) = obj.file() {
                        file
                    } else {
                        gio::File::for_path(glib::home_dir())
                    };

                    let file = crate::ui::utils::folder_chooser_dialog(
                        &gettext("Backup Location"),
                        Some(&preselect),
                    )
                    .await?;

                    obj.set_property("file", file);

                    Ok(())
                });
            });
        }
    }

    impl FolderButton {
        pub fn reset_label(&self) {
            self.label
                .set_text_with_mnemonic(&gettext("_Select Folder…"));
        }
    }

    impl WidgetImpl for FolderButton {}
    impl ButtonImpl for FolderButton {}
}
