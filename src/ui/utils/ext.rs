use chrono::prelude::*;
use gtk::prelude::*;
use libhandy::prelude::*;

pub trait CronoExt {
    fn to_locale(&self) -> Option<String>;
}

impl CronoExt for NaiveDateTime {
    fn to_locale(&self) -> Option<String> {
        let dt = chrono::Local.from_local_datetime(self).earliest()?;
        let gdt = glib::DateTime::from_unix_local(dt.timestamp());
        Some(gdt.ok()?.format("%c").ok()?.to_string())
    }
}

pub trait WidgetExtExt {
    fn add_css_class(&self, class: &str);
    fn remove_css_class(&self, class: &str);
}

impl<W: WidgetExt> WidgetExtExt for W {
    fn add_css_class(&self, class: &str) {
        self.style_context().add_class(class);
    }

    fn remove_css_class(&self, class: &str) {
        self.style_context().remove_class(class);
    }
}

pub trait ComboRowExtExt {
    fn selected<T: glib::IsA<glib::Object>>(&self) -> Option<T>;
}

impl<C: ComboRowExt> ComboRowExtExt for C {
    fn selected<T: glib::IsA<glib::Object>>(&self) -> Option<T> {
        self.model()
            .and_then(|model| {
                let index = self.selected_index();
                if index.is_negative() {
                    return None;
                }
                model.item(index as u32)
            })
            .and_then(|x| x.downcast::<T>().ok())
    }
}
