use adw::prelude::*;
use chrono::prelude::*;

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

pub trait ComboRowExtExt {
    fn selected_cast<T: glib::IsA<glib::Object>>(&self) -> Option<T>;
}

impl<C: ComboRowExt> ComboRowExtExt for C {
    fn selected_cast<T: glib::IsA<glib::Object>>(&self) -> Option<T> {
        self.selected_item().and_then(|x| x.downcast::<T>().ok())
    }
}
