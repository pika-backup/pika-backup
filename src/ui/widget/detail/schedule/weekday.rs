use glib::prelude::*;
use glib::subclass::prelude::*;
use std::cell::RefCell;

pub static LIST: [chrono::Weekday; 7] = [
    chrono::Weekday::Mon,
    chrono::Weekday::Tue,
    chrono::Weekday::Wed,
    chrono::Weekday::Thu,
    chrono::Weekday::Fri,
    chrono::Weekday::Sat,
    chrono::Weekday::Sun,
];

glib::wrapper! {
    pub struct WeekdayObject(ObjectSubclass<imp::WeekdayObject>);
}

impl WeekdayObject {
    pub fn new(weekday: chrono::Weekday) -> Self {
        let new: Self = glib::Object::new();
        new.imp().weekday.replace(weekday);
        new
    }

    pub fn weekday(&self) -> chrono::Weekday {
        *self.imp().weekday.borrow()
    }
}

mod imp {
    use std::marker::PhantomData;

    use super::*;

    #[derive(glib::Properties)]
    #[properties(wrapper_type = super::WeekdayObject)]
    pub struct WeekdayObject {
        pub weekday: RefCell<chrono::Weekday>,
        #[property(get = Self::name)]
        display: PhantomData<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WeekdayObject {
        const NAME: &'static str = "PikaBackupWeekday";
        type Type = super::WeekdayObject;
        type ParentType = glib::Object;
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
                display: Default::default(),
            }
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for WeekdayObject {}
}
