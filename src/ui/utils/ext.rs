use adw::prelude::*;
use chrono::TimeZone;

pub trait TimeExt {
    fn to_locale(&self) -> Option<String>;
}

impl TimeExt for chrono::NaiveDateTime {
    fn to_locale(&self) -> Option<String> {
        let dt = chrono::Local.from_local_datetime(self).earliest()?;
        let gdt = glib::DateTime::from_unix_local(dt.timestamp());
        Some(gdt.ok()?.format("%c").ok()?.to_string())
    }
}

impl TimeExt for time::PrimitiveDateTime {
    fn to_locale(&self) -> Option<String> {
        // FIXME: Use time::OffsetDateTime instead
        let dt = self.assume_utc();
        let gdt = glib::DateTime::from_unix_utc(dt.unix_timestamp());
        Some(gdt.ok()?.format("%c").ok()?.to_string())
    }
}

impl TimeExt for time::OffsetDateTime {
    fn to_locale(&self) -> Option<String> {
        // FIXME: Use time::OffsetDateTime instead
        let gdt = glib::DateTime::from_unix_utc(self.unix_timestamp());
        Some(gdt.ok()?.format("%c").ok()?.to_string())
    }
}

pub trait ToChronoLocal {
    fn to_chrono_local(&self) -> chrono::DateTime<chrono::Local>;
}

impl ToChronoLocal for time::OffsetDateTime {
    fn to_chrono_local(&self) -> chrono::DateTime<chrono::Local> {
        let timestamp_utc = self.unix_timestamp();
        let ndt = chrono::NaiveDateTime::from_timestamp(timestamp_utc, 0);
        ndt.and_local_timezone(chrono::offset::Local)
            .earliest()
            .unwrap()
    }
}

pub trait ToChronoPrimitive {
    fn to_chrono_primitive(&self) -> chrono::NaiveDateTime;
}

impl ToChronoPrimitive for time::PrimitiveDateTime {
    fn to_chrono_primitive(&self) -> chrono::NaiveDateTime {
        chrono::NaiveDateTime::from_timestamp(self.assume_utc().unix_timestamp(), 0)
    }
}

pub trait ToTime {
    fn to_time_local(&self) -> time::OffsetDateTime;
}

impl ToTime for chrono::DateTime<chrono::offset::Local> {
    fn to_time_local(&self) -> time::OffsetDateTime {
        let timestamp_utc = self.timestamp();

        // Doesn't really matter that we return UTC here, we can calculate with UTC just fine
        time::OffsetDateTime::from_unix_timestamp(timestamp_utc).unwrap()
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
