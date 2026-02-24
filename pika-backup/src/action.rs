//! These public actions are duplicates of the ones in the monitor.
//!
//! Since Flatpak sends shell notifications under the main app id, notifications
//! sent by the monitor, will go to the main app. If installed on the host, they
//! will go to the monitor.

use common::config::ConfigId;
use common::utils::action::Action;

use crate::globals::*;
use crate::status::QuitGuard;

pub struct StartBackup;

impl Action<String> for StartBackup {
    const NAME: &'static str = "start-backup";
    const PARAMETER_TYPE: Option<&glib::VariantTy> = Some(glib::VariantTy::STRING);

    fn activate(config_id: String) {
        // Prevent app from closing
        let guard = QuitGuard::default();
        // Start backup
        main_ui()
            .page_detail()
            .backup_page()
            .start_backup(ConfigId::new(config_id), None, guard);
    }
}

pub struct ShowOverview;

impl Action<()> for ShowOverview {
    const NAME: &'static str = "show-overview";

    fn activate(_: ()) {
        main_ui().page_overview().dbus_show();
    }
}

pub struct ShowSchedule;

impl Action<String> for ShowSchedule {
    const NAME: &'static str = "show-schedule";
    const PARAMETER_TYPE: Option<&glib::VariantTy> = Some(glib::VariantTy::STRING);

    fn activate(config_id: String) {
        main_ui()
            .page_detail()
            .schedule_page()
            .dbus_show(ConfigId::new(config_id));
    }
}
