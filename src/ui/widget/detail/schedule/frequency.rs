use crate::config;

use glib::prelude::*;
use glib::subclass::prelude::*;
use glib::{ParamSpec, ParamSpecString};
use std::cell::RefCell;
use std::sync::LazyLock;

pub fn list() -> Vec<config::Frequency> {
    vec![
        config::Frequency::Hourly,
        config::Frequency::Daily {
            preferred_time: chrono::NaiveTime::MIN,
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
        let new: Self = glib::Object::new();
        new.imp().frequency.replace(frequency);
        new
    }

    pub fn frequency(&self) -> config::Frequency {
        *self.imp().frequency.borrow()
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
            static PROPERTIES: LazyLock<Vec<ParamSpec>> =
                LazyLock::new(|| vec![ParamSpecString::builder("display").build()]);
            PROPERTIES.as_ref()
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> glib::Value {
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
