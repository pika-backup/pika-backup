use gtk::prelude::*;
use libhandy::prelude::*;

use super::event;
use super::frequency::{self, FrequencyObject};
use super::weekday::{self, WeekdayObject};
use crate::ui::prelude::*;

use once_cell::sync::Lazy;

pub(super) static SCHEDULE_ACTIVE_SIGNAL_HANDLER: Lazy<glib::SignalHandlerId> =
    Lazy::new(init_schedule_active);

pub fn init() {
    // frequency model

    let model = gio::ListStore::new(FrequencyObject::new(Default::default()).type_());

    for frequency in frequency::list() {
        model.append(&FrequencyObject::new(frequency));
    }

    main_ui()
        .schedule_frequency()
        .bind_name_model(Some(&model), Some(Box::new(frequency::name)));

    // weekday model

    let model = gio::ListStore::new(WeekdayObject::new(chrono::Weekday::Mon).type_());

    for weekday in &weekday::LIST {
        model.append(&WeekdayObject::new(*weekday));
    }

    main_ui()
        .preferred_weekday_row()
        .bind_name_model(Some(&model), Some(Box::new(weekday::name)));

    // events

    main_ui()
        .detail_stack()
        .connect_visible_child_notify(|_| Handler::run(event::show_page()));

    Lazy::force(&SCHEDULE_ACTIVE_SIGNAL_HANDLER);

    main_ui()
        .schedule_frequency()
        .connect_selected_index_notify(|_| Handler::run(event::frequency_change()));

    main_ui()
        .schedule_preferred_hour()
        .connect_output(event::preferred_time_change);

    main_ui()
        .schedule_preferred_minute()
        .connect_output(event::preferred_time_change);

    main_ui()
        .schedule_preferred_time_popover()
        .connect_closed(|_| Handler::run(event::preferred_time_close()));

    main_ui()
        .preferred_weekday_row()
        .connect_selected_index_notify(|_| Handler::run(event::preferred_weekday_change()));

    main_ui()
        .schedule_preferred_day_calendar()
        .connect_day_selected(|_| Handler::run(event::preferred_day_change()));
}

fn init_schedule_active() -> glib::SignalHandlerId {
    main_ui()
        .schedule_active()
        .connect_enable_expansion_notify(|_| Handler::run(event::active_change()))
}
