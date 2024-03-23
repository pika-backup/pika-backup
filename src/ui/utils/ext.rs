use adw::prelude::*;
use chrono::prelude::*;

use crate::prelude::*;

pub trait CronoExt {
    fn to_gnome_format(&self) -> Option<String>;
}

impl CronoExt for NaiveDateTime {
    fn to_gnome_format(&self) -> Option<String> {
        let dt = chrono::Local.from_local_datetime(self).earliest()?;
        let gdt = glib::DateTime::from_unix_local(dt.timestamp());
        // these match: (with show_weekday, show_full_date, show_seconds)
        // https://gitlab.gnome.org/GNOME/gnome-desktop/-/blob/ef825061211b/libgnome-desktop/gnome-wall-clock.c#L339
        let format = if *crate::globals::CLOCK_IS_24H {
            // Translators: This is the time format used for lists of full 
            // dates with times in 24-hour mode. It should use a numeric
            // date if possible to align longer lists of dates and times.
            gettext("%x %T")
        } else {
            // Translators: This is the time format used for lists of full 
            // dates with times in 12-hour mode (including AM/PM if
            // appropriate). It should use a numeric date if possible to
            // align longer lists of dates and times.
            gettext("%x %r")
        };
        Some(gdt.ok()?.format(&format).ok()?.to_string())
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
