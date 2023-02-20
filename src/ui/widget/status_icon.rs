use gtk::prelude::*;
use gtk::subclass::prelude::*;

use crate::ui::utils::StatusLevel;

glib::wrapper! {
    pub struct StatusIcon(ObjectSubclass<imp::StatusIcon>)
        @extends gtk::Box, gtk::Widget;
}

impl StatusIcon {
    pub fn new(icon_name: &str, level: StatusLevel) -> Self {
        let obj: Self = glib::Object::new();

        obj.set_icon_name(icon_name);
        obj.set_level(level);

        obj
    }

    pub fn set_icon_name(&self, icon_name: &str) {
        self.imp().image.set_from_icon_name(Some(icon_name));
    }

    pub fn set_level(&self, level: StatusLevel) {
        match level {
            StatusLevel::Ok => self.set_ok(),
            StatusLevel::Neutral => self.set_neutral(),
            StatusLevel::Warning => self.set_warning(),
            StatusLevel::Error => self.set_error(),
        }
    }

    pub fn set_neutral(&self) {
        let imp = self.imp();

        imp.image.remove_css_class("ok-icon");
        imp.image.remove_css_class("warning-icon");
        imp.image.remove_css_class("error-icon");
    }

    pub fn set_ok(&self) {
        let imp = self.imp();
        imp.image.add_css_class("ok-icon");

        imp.image.remove_css_class("warning-icon");
        imp.image.remove_css_class("error-icon");
    }

    pub fn set_warning(&self) {
        let imp = self.imp();
        imp.image.add_css_class("warning-icon");

        imp.image.remove_css_class("ok-icon");
        imp.image.remove_css_class("error-icon");
    }

    pub fn set_error(&self) {
        let imp = self.imp();
        imp.image.add_css_class("error-icon");

        imp.image.remove_css_class("ok-icon");
        imp.image.remove_css_class("warning-icon");
    }
}

mod imp {
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;

    use once_cell::sync::Lazy;

    #[derive(Debug, Default)]
    pub struct StatusIcon {
        pub image: Lazy<gtk::Image>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StatusIcon {
        const NAME: &'static str = "PikaStatusIcon";
        type Type = super::StatusIcon;
        type ParentType = gtk::Box;
    }

    impl ObjectImpl for StatusIcon {
        fn constructed(&self) {
            self.image.add_css_class("status-icon");
            self.image.set_valign(gtk::Align::Center);

            self.obj().append(&*self.image);
        }
    }

    impl WidgetImpl for StatusIcon {}
    impl BoxImpl for StatusIcon {}
}
