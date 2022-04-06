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
        let max_width = 350;
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
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;

    use std::cell::RefCell;

    #[derive(Debug, Default)]
    pub struct WrapBox {
        pub children: RefCell<Vec<gtk::Widget>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WrapBox {
        const NAME: &'static str = "PikaWrapBox";
        type Type = super::WrapBox;
        type ParentType = gtk::Box;
    }

    impl ObjectImpl for WrapBox {
        fn constructed(&self, obj: &Self::Type) {
            obj.set_orientation(gtk::Orientation::Vertical);
            obj.set_hexpand(true);
        }
    }

    impl WidgetImpl for WrapBox {}
    impl BoxImpl for WrapBox {}
    impl OrientableImpl for WrapBox {}
}
