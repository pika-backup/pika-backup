use std::cell::{Cell, OnceCell};

use adw::prelude::*;
use adw::subclass::prelude::*;
use adw::{glib, gtk};

mod imp {

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::ValueUnit)]
    #[template(file = "value_unit.ui")]
    pub struct ValueUnit {
        #[property(get, set=Self::set_value)]
        value: Cell<u64>,
        #[property(set, construct_only)]
        unit_size_group: OnceCell<Option<gtk::SizeGroup>>,
        #[property(set, construct_only)]
        value_size_group: OnceCell<Option<gtk::SizeGroup>>,

        #[template_child]
        value_label: TemplateChild<gtk::Label>,
        #[template_child]
        unit_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ValueUnit {
        const NAME: &'static str = "PkSizeEstimateValueUnit";
        type Type = super::ValueUnit;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ValueUnit {
        fn constructed(&self) {
            self.parent_constructed();

            if let Some(group) = self.unit_size_group.get().unwrap() {
                group.add_widget(&*self.unit_label);
            }

            if let Some(group) = self.value_size_group.get().unwrap() {
                group.add_widget(&*self.value_label);
            }
        }
    }

    impl WidgetImpl for ValueUnit {}
    impl BoxImpl for ValueUnit {}

    impl ValueUnit {
        fn set_value(&self, value: u64) {
            self.value.replace(value);

            let s_value = glib::format_size_full(value, glib::FormatSizeFlags::ONLY_VALUE);
            let s_unit = glib::format_size_full(value, glib::FormatSizeFlags::ONLY_UNIT);

            self.value_label.set_label(&s_value);
            self.unit_label.set_label(&s_unit);
        }
    }
}

glib::wrapper! {
    pub struct ValueUnit(ObjectSubclass<imp::ValueUnit>)
    @extends gtk::Widget, gtk::Box,
    @implements gtk::ConstraintTarget, gtk::Buildable, gtk::Accessible;
}

impl ValueUnit {
    pub fn new(value_size_group: gtk::SizeGroup, unit_size_group: gtk::SizeGroup) -> Self {
        glib::Object::builder()
            .property("value_size_group", value_size_group)
            .property("unit_size_group", unit_size_group)
            .build()
    }
}
