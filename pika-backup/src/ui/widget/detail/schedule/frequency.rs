use std::cell::RefCell;

use glib::prelude::*;
use glib::subclass::prelude::*;

use crate::config;

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
    use std::marker::PhantomData;

    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type=super::FrequencyObject)]
    pub struct FrequencyObject {
        pub frequency: RefCell<config::Frequency>,
        #[property(get=Self::name)]
        display: PhantomData<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FrequencyObject {
        const NAME: &'static str = "PikaBackupScheduleFrequency";
        type Type = super::FrequencyObject;
        type ParentType = glib::Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for FrequencyObject {}

    impl FrequencyObject {
        fn name(&self) -> String {
            self.frequency.borrow().name()
        }
    }
}
