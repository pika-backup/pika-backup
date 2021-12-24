use crate::config;

use glib::prelude::*;
use glib::subclass::prelude::*;
use glib::{ParamFlags, ParamSpec, ParamSpecString};
use once_cell::sync::Lazy;
use std::cell::RefCell;

pub fn list() -> Vec<config::Frequency> {
    vec![
        config::Frequency::Hourly,
        config::Frequency::Daily {
            preferred_time: chrono::NaiveTime::from_hms(0, 0, 0),
        },
        config::Frequency::Weekly {
            preferred_weekday: chrono::Weekday::Mon,
        },
        config::Frequency::Monthly { preferred_day: 1 },
    ]
}

glib::wrapper! {
    pub struct FrequencyObject(ObjectSubclass<imp::FrequencyObject>);
}

impl FrequencyObject {
    pub fn new(frequency: config::Frequency) -> Self {
        let new: Self = glib::Object::new(&[]).unwrap();
        let priv_ = imp::FrequencyObject::from_instance(&new);
        priv_.frequency.replace(frequency);
        new
    }

    pub fn frequency(&self) -> config::Frequency {
        let priv_ = imp::FrequencyObject::from_instance(self);
        (*priv_.frequency.borrow()).clone()
    }
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct FrequencyObject {
        pub frequency: RefCell<config::Frequency>,
    }

    impl ObjectImpl for FrequencyObject {
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
                "display" => self.frequency.borrow().name().to_value(),
                _ => unimplemented!(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FrequencyObject {
        const NAME: &'static str = "PikaBackupScheduleFrequency";
        type Type = super::FrequencyObject;
        type ParentType = glib::Object;
    }
}
