use crate::ui::backup_status::Graphic;
use crate::ui::utils::StatusLevel;

glib::wrapper! {
    pub struct StatusIcon(ObjectSubclass<imp::StatusIcon>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;

}

impl StatusIcon {
    pub fn new(icon_name: &str, level: StatusLevel) -> Self {
        glib::Object::builder()
            .property("icon-name", icon_name)
            .property("level", level)
            .build()
    }

    pub fn set_from_graphic(&self, graphic: &Graphic) {
        match graphic {
            Graphic::OkIcon(icon) => {
                self.set_icon_name(icon.to_owned());
                self.set_ok();
            }
            Graphic::WarningIcon(icon) => {
                self.set_icon_name(icon.to_owned());
                self.set_warning();
            }
            Graphic::ErrorIcon(icon) => {
                self.set_icon_name(icon.to_owned());
                self.set_error();
            }
            Graphic::Spinner => {
                self.set_spinner();
            }
        }
    }

    pub fn set_neutral(&self) {
        self.set_level(StatusLevel::Neutral);
    }

    pub fn set_ok(&self) {
        self.set_level(StatusLevel::Ok);
    }

    pub fn set_warning(&self) {
        self.set_level(StatusLevel::Warning);
    }

    pub fn set_error(&self) {
        self.set_level(StatusLevel::Error);
    }

    pub fn set_spinner(&self) {
        self.set_level(StatusLevel::Spinner);
    }
}

impl Default for StatusIcon {
    fn default() -> Self {
        Self::new("", StatusLevel::Neutral)
    }
}

mod imp {
    use std::cell::{Cell, RefCell};

    use gtk::prelude::*;
    use gtk::subclass::prelude::*;

    use crate::ui::utils::StatusLevel;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::StatusIcon)]
    pub struct StatusIcon {
        pub image: gtk::Image,
        pub spinner: adw::Spinner,
        pub stack: gtk::Stack,

        #[property(get, set)]
        icon_name: RefCell<String>,
        #[property(get, set = Self::set_level, explicit_notify)]
        level: Cell<StatusLevel>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StatusIcon {
        const NAME: &'static str = "PkStatusIcon";
        type Type = super::StatusIcon;
        type ParentType = gtk::Box;
    }

    #[glib::derived_properties]
    impl ObjectImpl for StatusIcon {
        fn constructed(&self) {
            self.parent_constructed();

            self.image.add_css_class("status-icon");
            self.image.set_valign(gtk::Align::Center);
            self.obj()
                .bind_property("icon-name", &self.image, "icon-name")
                .build();

            self.spinner.add_css_class("row-icon");
            self.spinner.set_width_request(24);
            self.spinner.set_height_request(24);
            self.spinner.set_valign(gtk::Align::Center);
            self.spinner.set_halign(gtk::Align::Center);

            self.stack.add_child(&self.image);
            self.stack.add_child(&self.spinner);
            self.stack.set_visible_child(&self.image);

            self.obj().append(&self.stack);
        }
    }

    impl WidgetImpl for StatusIcon {}
    impl BoxImpl for StatusIcon {}

    impl StatusIcon {
        pub(super) fn show_icon(&self) {
            self.stack.set_visible_child(&self.image);
        }

        pub(super) fn show_spinner(&self) {
            self.stack.set_visible_child(&self.spinner);
        }

        pub fn set_level(&self, level: StatusLevel) {
            if level == self.level.get() {
                return;
            }

            self.image.remove_css_class("ok-icon");
            self.image.remove_css_class("warning-icon");
            self.image.remove_css_class("error-icon");
            self.show_icon();

            match level {
                StatusLevel::Ok => self.image.add_css_class("ok-icon"),
                StatusLevel::Warning => self.image.add_css_class("warning-icon"),
                StatusLevel::Error => self.image.add_css_class("error-icon"),
                StatusLevel::Neutral => {}
                StatusLevel::Spinner => self.show_spinner(),
            }

            self.level.replace(level);
            self.obj().notify_level();
        }
    }
}
