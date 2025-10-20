use adw::prelude::*;
use adw::subclass::prelude::*;
use enclose::enclose;

use crate::ui::widget::StatusRow;

mod imp {
    use std::cell::OnceCell;

    use super::*;
    use crate::ui::widget::{StatusRow, WrapBox};
    use crate::ui::{self, App};

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[template(file = "row.ui")]
    #[properties(wrapper_type = super::OverviewRow)]
    pub struct OverviewRow {
        #[property(get, set, construct_only)]
        config: OnceCell<common::config::Backup>,

        #[template_child]
        location_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        location_icon: TemplateChild<gtk::Image>,
        #[template_child]
        location_title: TemplateChild<gtk::Label>,
        #[template_child]
        location_subtitle: TemplateChild<gtk::Label>,
        #[template_child]
        include_box: TemplateChild<WrapBox>,
        #[template_child]
        pub(super) status: TemplateChild<StatusRow>,
        #[template_child]
        pub(super) schedule_status: TemplateChild<StatusRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for OverviewRow {
        const NAME: &'static str = "PkOverviewRow";
        type Type = super::OverviewRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for OverviewRow {
        fn constructed(&self) {
            self.parent_constructed();
            let config = self
                .config
                .get()
                .expect("construct_only property must be set");

            // connect click
            self.location_row
                .connect_activated(enclose!((config) move |_| {
                    let window = App::default().main_window();
                    window.view_backup_conf(&config.id);
                }));

            self.schedule_status
                .connect_activated(enclose!((config) move |_| {
                    let window = App::default().main_window();
                    window.page_detail().schedule_page().view(&config.id);
                }));

            // Repo Icon
            if let Ok(icon) = gio::Icon::for_string(&config.repo.icon()) {
                self.location_icon.set_from_gicon(&icon);
            }

            // Repo Name
            self.location_title.set_label(&config.title());
            self.location_subtitle.set_label(&config.repo.subtitle());

            // Include

            for path in &config.include {
                let incl = ui::widget::LocationTag::from_path(path.clone());

                self.include_box.add_child(&incl.build());
            }
        }
    }
    impl WidgetImpl for OverviewRow {}
    impl ListBoxRowImpl for OverviewRow {}

    #[gtk::template_callbacks]
    impl OverviewRow {}
}

glib::wrapper! {
    pub struct OverviewRow(ObjectSubclass<imp::OverviewRow>)
    @extends gtk::Widget, gtk::ListBoxRow,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Actionable;
}

impl OverviewRow {
    pub fn new(config: &common::config::Backup) -> Self {
        glib::Object::builder().property("config", config).build()
    }

    pub fn status(&self) -> StatusRow {
        self.imp().status.clone()
    }

    pub fn schedule_status(&self) -> StatusRow {
        self.imp().schedule_status.clone()
    }
}
