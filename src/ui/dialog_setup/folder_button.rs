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
        glib::Object::new(&[]).unwrap()
    }

    pub fn file(&self) -> Option<gio::File> {
        let priv_ = imp::FolderButton::from_instance(self);
        (*priv_.file.borrow()).clone()
    }

    pub fn connect_folder_change<F: 'static + Fn()>(&self, f: F) -> gtk::glib::SignalHandlerId {
        self.connect_notify_local(Some("file"), move |_, _| f())
    }
}

mod imp {
    use crate::ui::prelude::*;
    use glib::{ParamFlags, ParamSpec, ParamSpecObject, Value};
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use once_cell::sync::Lazy;
    use std::cell::RefCell;

    #[derive(Default)]
    pub struct FolderButton {
        pub file: RefCell<Option<gio::File>>,
        pub file_chooser: RefCell<Option<gtk::FileChooserNative>>,
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
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![ParamSpecObject::new(
                    "file",
                    "file",
                    "file",
                    gio::File::static_type(),
                    ParamFlags::READWRITE,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "file" => self.file.borrow().to_value(),
                _ => unimplemented!(),
            }
        }

        fn set_property(&self, _obj: &Self::Type, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "file" => {
                    let file = value.get::<gio::File>().ok();

                    let info = file.as_ref().and_then(|x| {
                        x.query_info(
                            "standard::*",
                            gtk::gio::FileQueryInfoFlags::NONE,
                            gtk::gio::Cancellable::NONE,
                        )
                        .ok()
                    });

                    let mount_icon = file
                        .as_ref()
                        .and_then(|x| x.find_enclosing_mount(gtk::gio::Cancellable::NONE).ok())
                        .map(|x| x.icon());
                    let file_icon = info.as_ref().and_then(|x| x.icon());

                    self.image
                        .set_gicon([mount_icon, file_icon].iter().flatten().next());
                    self.image.show();

                    self.label.set_label(
                        &info
                            .map(|x| x.display_name().to_string())
                            .unwrap_or_default(),
                    );

                    self.file.replace(file);
                }
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            obj.set_child(Some(&*self.child));
            obj.add_css_class("folder-button");

            self.child.append(&*self.image);
            self.image.hide();

            self.child.append(&*self.label);
            self.label.set_label(&gettext("Select…"));

            let objc = obj.clone();
            obj.connect_clicked(move |_| {
                let dialog = crate::ui::utils::folder_chooser(
                    &gettext("Backup Location"),
                    &objc
                        .root()
                        .and_then(|x| x.downcast::<gtk::Window>().ok())
                        .unwrap(),
                );

                let priv_ = Self::from_instance(&objc);
                *priv_.file_chooser.borrow_mut() = Some(dialog.clone());

                let obj = objc.clone();

                dialog.connect_response(move |file_chooser, s| {
                    if s == gtk::ResponseType::Accept {
                        if let Some(file) = file_chooser.file() {
                            obj.set_property("file", file);
                        }
                    }
                });

                dialog.show();
            });
        }
    }

    impl WidgetImpl for FolderButton {}
    impl ButtonImpl for FolderButton {}
}
