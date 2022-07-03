use gtk::prelude::*;
use gtk::subclass::prelude::*;

glib::wrapper! {
    pub struct WrapBox(ObjectSubclass<imp::WrapBox>)
        @extends gtk::Box, gtk::Widget, gtk::Orientable;
}

impl WrapBox {
    pub fn new() -> Self {
        glib::Object::new(&[]).unwrap()
    }

    pub fn add_child<T: IsA<gtk::Widget>>(&self, child: &T) {
        self.imp()
            .children
            .borrow_mut()
            .push(child.clone().upcast());

        self.rebuild();
    }

    pub fn rebuild(&self) {
        let max_width = self.imp().width_estimate.get();
        let spacing = 4;

        while let Some(hbox) = self
            .last_child()
            .and_then(|x| x.downcast::<gtk::Box>().ok())
        {
            while let Some(child) = hbox.last_child() {
                hbox.remove(&child);
            }
            self.remove(&hbox);
        }

        let mut hbox = gtk::Box::builder().hexpand(true).spacing(spacing).build();
        self.append(&hbox);

        let mut cur_width = 0;

        for child in self.imp().children.borrow().iter() {
            let (_, natural_width, _, _) = child.measure(gtk::Orientation::Horizontal, -1);
            let mut this_width = spacing + natural_width;
            if this_width > max_width / 2 {
                this_width = max_width / 2;
            }

            cur_width += this_width;
            if cur_width > max_width {
                hbox = gtk::Box::builder().spacing(spacing).build();
                self.append(&hbox);
                cur_width = this_width;
            }
            hbox.append(child);
        }
    }
}

mod imp {
    use crate::ui::prelude::*;

    use glib::{ParamSpec, ParamSpecInt, Value};
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;

    use once_cell::sync::Lazy;
    use std::cell::{Cell, RefCell};

    #[derive(Debug, Default)]
    pub struct WrapBox {
        pub children: RefCell<Vec<gtk::Widget>>,
        pub width_estimate: Cell<i32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WrapBox {
        const NAME: &'static str = "PikaWrapBox";
        type Type = super::WrapBox;
        type ParentType = gtk::Box;
    }

    impl ObjectImpl for WrapBox {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![ParamSpecInt::builder("width-estimate")
                    .minimum(100)
                    .maximum(1000)
                    .default_value(350)
                    .build()]
            });
            PROPERTIES.as_ref()
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "width-estimate" => self.width_estimate.get().to_value(),
                _ => unimplemented!(),
            }
        }

        fn set_property(&self, _obj: &Self::Type, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "width-estimate" => {
                    if let Ok(width) = value.get() {
                        self.width_estimate.set(width);
                    } else {
                        error!("Invalid value for property width-estimate");
                    }
                }
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            obj.set_orientation(gtk::Orientation::Vertical);
            obj.set_hexpand(true);
            self.width_estimate.set(350);
        }
    }

    impl WidgetImpl for WrapBox {}
    impl BoxImpl for WrapBox {}
    impl OrientableImpl for WrapBox {}
}
