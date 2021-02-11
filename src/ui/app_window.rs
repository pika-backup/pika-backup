use crate::ui::prelude::*;
use gtk::prelude::*;

pub fn init() {
    main_ui().window().connect_delete_event(|_, _| on_delete());

    // decorate headerbar of pre-release versions
    if !option_env!("APPLICATION_ID_SUFFIX")
        .unwrap_or_default()
        .is_empty()
    {
        main_ui().window().get_style_context().add_class("devel");
    }
}

pub fn is_displayed() -> bool {
    main_ui().window().get_id() != 0
}

fn on_delete() -> Inhibit {
    debug!("Potential quit: ApplicationWindow delete event");

    Handler::run(super::quit());
    Inhibit(true)
}
