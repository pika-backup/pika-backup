use glib::prelude::*;
use glib::subclass::prelude::*;
use std::cell::RefCell;

use glib::{ParamFlags, ParamSpec, ParamSpecString};
use once_cell::sync::Lazy;

pub static LIST: [chrono::Weekday; 7] = [
    chrono::Weekday::Mon,
    chrono::Weekday::Tue,
    chrono::Weekday::Wed,
    chrono::Weekday::Thu,
    chrono::Weekday::Fri,
    chrono::Weekday::Sat,
    chrono::Weekday::Sun,
];

/*
pub fn name(obj: &glib::Object) -> String {
    obj.downcast_ref::<WeekdayObject>()
        .and_then(|obj| {
            glib::DateTime::new_local(2021, 3, obj.weekday().number_from_monday() as i32, 0, 0, 0.)
                .ok()
        })
        .and_then(|dt| dt.format("%A").ok())
        .map(|x| x.to_string())
        .unwrap_or_default()
}
*/

glib::wrapper! {
    pub struct WeekdayObject(ObjectSubclass<imp::WeekdayObject>);
}

impl WeekdayObject {
    pub fn new(weekday: chrono::Weekday) -> Self {
        let new: Self = glib::Object::new(&[]).unwrap();
        let priv_ = imp::WeekdayObject::from_instance(&new);
        priv_.weekday.replace(weekday);
        new
    }

    pub fn weekday(&self) -> chrono::Weekday {
        let priv_ = imp::WeekdayObject::from_instance(self);
        *priv_.weekday.borrow()
    }
}

mod imp {
    use super::*;

    pub struct WeekdayObject {
        pub weekday: RefCell<chrono::Weekday>,
    }

    impl WeekdayObject {
        pub fn name(&self) -> String {
            glib::DateTime::from_local(
                2021,
                3,
                self.weekday.borrow().number_from_monday() as i32,
                0,
                0,
                0.,
            )
            .ok()
            .and_then(|dt| dt.format("%A").ok())
            .map(|x| x.to_string())
            .unwrap_or_default()
        }
    }

    impl Default for WeekdayObject {
        fn default() -> Self {
            Self {
                weekday: RefCell::new(chrono::Weekday::Fri),
            }
        }
    }

    impl ObjectImpl for WeekdayObject {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![ParamSpecString::new(
                    "display",
                    "display",
                    "display",
                    None,
                    ParamFlags::READABLE,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "display" => self.name().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WeekdayObject {
        const NAME: &'static str = "PikaBackupWeekday";
        type Type = super::WeekdayObject;
        type ParentType = glib::Object;
    }
}
