use adw::prelude::*;
use adw::subclass::prelude::*;

use crate::backup_status;
use crate::utils::StatusLevel;

mod imp {
    use std::cell::{Cell, RefCell};

    use adw::prelude::*;
    use adw::subclass::prelude::*;

    use crate::utils::StatusLevel;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::StatusRow)]
    pub struct StatusRow {
        #[property(get, set)]
        icon_name: RefCell<String>,
        #[property(get, set)]
        level: Cell<StatusLevel>,
        pub(super) status_icon: crate::widget::StatusIcon,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StatusRow {
        const NAME: &'static str = "PikaStatusRow";
        type Type = super::StatusRow;
        type ParentType = adw::ActionRow;
    }

    impl ObjectImpl for StatusRow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.add_prefix(&self.status_icon);

            obj.set_subtitle_lines(1);

            obj.add_css_class("numeric");

            obj.bind_property("icon-name", &self.status_icon, "icon-name")
                .build();
            obj.bind_property("level", &self.status_icon, "level")
                .build();
        }

        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }
        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }
        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }
    }
    impl WidgetImpl for StatusRow {}
    impl ListBoxRowImpl for StatusRow {}
    impl PreferencesRowImpl for StatusRow {}
    impl ActionRowImpl for StatusRow {}
}

glib::wrapper! {
    pub struct StatusRow(ObjectSubclass<imp::StatusRow>)
        @extends adw::ActionRow, adw::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Actionable;
}

impl StatusRow {
    pub fn new(
        title: impl AsRef<str>,
        subtitle: impl AsRef<str>,
        icon_name: impl AsRef<str>,
        level: StatusLevel,
    ) -> Self {
        glib::Object::builder()
            .property("title", title.as_ref())
            .property("subtitle", subtitle.as_ref())
            .property("icon-name", icon_name.as_ref())
            .property("level", level)
            .build()
    }

    pub fn set_from_backup_status(&self, status: &backup_status::Display) {
        self.set_title(&glib::markup_escape_text(&status.title));
        self.set_subtitle(&glib::markup_escape_text(
            status.subtitle.as_deref().unwrap_or(""),
        ));
        self.imp().status_icon.set_from_graphic(&status.graphic);
    }
}
