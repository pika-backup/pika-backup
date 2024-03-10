mod event;
pub mod frequency;
pub mod prune_preset;
pub mod status;
pub mod weekday;

use frequency::FrequencyObject;
use prune_preset::PrunePresetObject;
use weekday::WeekdayObject;

use crate::ui::prelude::*;
use adw::prelude::*;
use adw::subclass::prelude::*;

use super::detail_page::DetailPageKind;

mod imp {
    use std::cell::OnceCell;

    use crate::ui::widget::StatusRow;

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(file = "schedule_page.ui")]
    pub struct SchedulePage {
        // Status
        #[template_child]
        pub(super) status_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) status_row: TemplateChild<StatusRow>,

        // Configure Schedule
        #[template_child]
        pub(super) schedule_active: TemplateChild<adw::ExpanderRow>,
        #[template_child]
        pub(super) frequency: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub(super) preferred_time_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) preferred_time_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) preferred_weekday_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub(super) preferred_day: TemplateChild<adw::SpinRow>,

        // Preferred time popover
        #[template_child]
        pub(super) preferred_time_popover: TemplateChild<gtk::Popover>,
        #[template_child]
        pub(super) preferred_hour: TemplateChild<gtk::SpinButton>,
        #[template_child]
        pub(super) preferred_minute: TemplateChild<gtk::SpinButton>,

        // Prune
        #[template_child]
        pub(super) prune_save_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub(super) prune_save: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) prune_enabled: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) prune_preset: TemplateChild<adw::ComboRow>,

        // Prune detail
        #[template_child]
        pub(super) prune_detail: TemplateChild<adw::ExpanderRow>,
        #[template_child]
        pub(super) keep_hourly: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub(super) keep_daily: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub(super) keep_weekly: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub(super) keep_monthly: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub(super) keep_yearly: TemplateChild<adw::SpinRow>,

        // Misc
        pub(super) schedule_active_signal_handler: OnceCell<glib::SignalHandlerId>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SchedulePage {
        const NAME: &'static str = "PkSchedulePage";
        type Type = super::SchedulePage;
        type ParentType = adw::PreferencesPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SchedulePage {
        fn constructed(&self) {
            let imp = self.ref_counted();

            // frequency model
            let model = gio::ListStore::with_type(FrequencyObject::static_type());

            for frequency in frequency::list() {
                model.append(&FrequencyObject::new(frequency));
            }

            self.frequency.set_model(Some(&model));

            // weekday model

            let model = gio::ListStore::with_type(WeekdayObject::static_type());

            for weekday in &weekday::LIST {
                model.append(&WeekdayObject::new(*weekday));
            }

            self.preferred_weekday_row.set_model(Some(&model));

            // events

            self.schedule_active_signal_handler.get_or_init(|| {
                self.schedule_active.connect_enable_expansion_notify(
                    glib::clone!(@weak imp => move |_| Handler::run(async move { imp.active_change().await })),
                )
            });

            self.frequency.connect_selected_item_notify(
                glib::clone!(@weak imp => move |_| Handler::run(async move { imp.frequency_change().await })),
            );

            self.preferred_hour.connect_output(
                glib::clone!(@weak imp => @default-return glib::Propagation::Stop, move |button| {
                    imp.preferred_time_change(button)
                }),
            );

            self.preferred_minute.connect_output(
                glib::clone!(@weak imp => @default-return glib::Propagation::Stop, move |button| {
                    imp.preferred_time_change(button)
                }),
            );

            self.preferred_time_popover.connect_closed(
                glib::clone!(@weak imp => move |_| Handler::run(async move { imp.preferred_time_close().await })),
            );

            self.preferred_weekday_row.connect_selected_item_notify(
                glib::clone!(@weak imp => move |_| Handler::run(async move { imp.preferred_weekday_change().await })),
            );

            self.preferred_day
                .connect_value_notify(glib::clone!(@weak imp => move |_| Handler::run(async move { imp.preferred_day_change().await })));

            // prune

            self.prune_save
                .connect_clicked(glib::clone!(@weak imp => move |_| Handler::run(async move { imp.prune_save().await })));

            self.prune_enabled
                .connect_active_notify(glib::clone!(@weak imp => move |_| Handler::run(async move { imp.prune_enabled().await } )));

            self.prune_preset
                .set_model(Some(&PrunePresetObject::list_store()));

            self.prune_preset
                .connect_selected_item_notify(glib::clone!(@weak imp => move |_| Handler::run(async move { imp.prune_preset_change().await } )));

            self.keep_hourly
                .connect_value_notify(glib::clone!(@weak imp => move |_| Handler::run(async move { imp.keep_change().await } )));

            self.keep_daily
                .connect_value_notify(glib::clone!(@weak imp => move |_| Handler::run(async move { imp.keep_change().await } )));

            self.keep_weekly
                .connect_value_notify(glib::clone!(@weak imp => move |_| Handler::run(async move { imp.keep_change().await } )));

            self.keep_monthly
                .connect_value_notify(glib::clone!(@weak imp => move |_| Handler::run(async move { imp.keep_change().await } )));

            self.keep_yearly
                .connect_value_notify(glib::clone!(@weak imp => move |_| Handler::run(async move { imp.keep_change().await } )));

            // Network

            gio::NetworkMonitor::default()
                .connect_network_metered_notify(glib::clone!(@weak imp => move |_| Handler::run(async move { imp.network_changed().await } )));

            gio::NetworkMonitor::default()
                .connect_network_available_notify(glib::clone!(@weak imp => move |_| Handler::run(async move { imp.network_changed().await } )));

            glib::timeout_add_local_once(std::time::Duration::ZERO, move || {
                // TODO: This should be run directly, but as long as we need main_ui we need to do it later to prevent recursion
                main_ui().navigation_view().connect_visible_page_notify(
                    glib::clone!(@weak imp => move |_| Handler::run(async move { imp.show_page().await })),
                );
            });
        }
    }

    impl WidgetImpl for SchedulePage {}
    impl PreferencesPageImpl for SchedulePage {}

    #[gtk::template_callbacks]
    impl SchedulePage {}
}

glib::wrapper! {
    pub struct SchedulePage(ObjectSubclass<imp::SchedulePage>)
    @extends adw::PreferencesPage, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SchedulePage {
    pub fn dbus_show(&self, id: ConfigId) {
        self.view(&id);
        adw_app().activate();
    }

    pub fn refresh(&self) {
        let obj = self.clone();
        Handler::run(async move { obj.imp().show_page().await });
    }

    pub fn view(&self, id: &ConfigId) {
        ACTIVE_BACKUP_ID.update(|active_id| *active_id = Some(id.clone()));

        main_ui().navigation_view().push(&main_ui().page_detail());
        main_ui()
            .page_detail()
            .show_stack_page(DetailPageKind::Schedule);
    }

    pub fn is_visible(&self) -> bool {
        main_ui().page_detail().visible_stack_page() == DetailPageKind::Schedule
    }

    pub fn refresh_status(&self) {
        if self.is_visible() {
            if let Ok(config) = BACKUP_CONFIG.load().active().cloned() {
                let obj = self.clone();
                glib::MainContext::default()
                    .spawn_local(async move { obj.imp().update_status(&config).await });
            }
        }
    }
}
