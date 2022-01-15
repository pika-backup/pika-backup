use crate::ui;
use glib::prelude::*;
use ui::utils::StatusLevel;

pub fn init() {
    ui::page_schedule::frequency::FrequencyObject::static_type();
    ui::page_schedule::prune_preset::PrunePresetObject::static_type();
    ui::page_schedule::weekday::WeekdayObject::static_type();
    ui::dialog_setup::folder_button::FolderButton::static_type();
    ui::dialog_setup::add_task::AddConfigTask::static_type();
    ui::widgets::StatusIcon::static_type();
}

use gtk::prelude::*;
use gtk::subclass::prelude::*;

glib::wrapper! {
    pub struct StatusIcon(ObjectSubclass<imp::StatusIcon>)
        @extends gtk::Box, gtk::Widget;
}

impl StatusIcon {
    pub fn new(icon_name: &str, level: StatusLevel) -> Self {
        let obj: Self = glib::Object::new(&[]).unwrap();

        obj.set_icon_name(icon_name);
        obj.set_level(level);

        obj
    }

    pub fn set_icon_name(&self, icon_name: &str) {
        let priv_ = imp::StatusIcon::from_instance(self);
        priv_.image.set_from_icon_name(Some(icon_name));
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
        let priv_ = imp::StatusIcon::from_instance(self);

        priv_.image.remove_css_class("ok-icon");
        priv_.image.remove_css_class("warning-icon");
        priv_.image.remove_css_class("error-icon");
    }

    pub fn set_ok(&self) {
        let priv_ = imp::StatusIcon::from_instance(self);
        priv_.image.add_css_class("ok-icon");

        priv_.image.remove_css_class("warning-icon");
        priv_.image.remove_css_class("error-icon");
    }

    pub fn set_warning(&self) {
        let priv_ = imp::StatusIcon::from_instance(self);
        priv_.image.add_css_class("warning-icon");

        priv_.image.remove_css_class("ok-icon");
        priv_.image.remove_css_class("error-icon");
    }

    pub fn set_error(&self) {
        let priv_ = imp::StatusIcon::from_instance(self);
        priv_.image.add_css_class("error-icon");

        priv_.image.remove_css_class("ok-icon");
        priv_.image.remove_css_class("warning-icon");
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
        fn constructed(&self, obj: &Self::Type) {
            self.image.add_css_class("status-icon");
            self.image.set_valign(gtk::Align::Center);

            obj.append(&*self.image);
        }
    }

    impl WidgetImpl for StatusIcon {}
    impl BoxImpl for StatusIcon {}
}
