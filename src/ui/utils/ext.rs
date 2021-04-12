use chrono::prelude::*;
use gtk::prelude::*;

pub trait CronoExt {
    fn to_locale(&self) -> Option<String>;
}

impl CronoExt for NaiveDateTime {
    fn to_locale(&self) -> Option<String> {
        let dt = chrono::Local.from_local_datetime(&self).earliest()?;
        let gdt = glib::DateTime::from_unix_local(dt.timestamp());
        Some(gdt.unwrap().format("%c").ok()?.to_string())
    }
}

pub trait WidgetExtExt {
    fn add_css_class(&self, class: &str);
    fn remove_css_class(&self, class: &str);
}

impl<W: gtk::WidgetExt> WidgetExtExt for W {
    fn add_css_class(&self, class: &str) {
        self.get_style_context().add_class(class);
    }

    fn remove_css_class(&self, class: &str) {
        self.get_style_context().remove_class(class);
    }
}
