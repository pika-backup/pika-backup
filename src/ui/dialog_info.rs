use gtk::prelude::*;

use num_format::ToFormattedString;

use crate::borg;
use crate::config::history::*;
use crate::ui::backup_status;
use crate::ui::prelude::*;

pub fn init() {
    glib::timeout_add_local(Duration::from_millis(250), || {
        refresh_status();
        Continue(true)
    });
}

fn is_visible() -> bool {
    main_ui().detail_running_backup_info().is_visible()
}

fn refresh_status() {
    if is_visible() {
        if let Some(id) = ACTIVE_BACKUP_ID.load().as_ref().as_ref() {
            refresh_status_display(&backup_status::Display::new_from_id(id));
        }
    }
}

fn refresh_status_display(status: &backup_status::Display) {
    main_ui().detail_info_status().set_text(&status.title);

    if let Some(progress) = status.progress {
        main_ui().detail_info_progress().set_fraction(progress);
        main_ui().detail_info_progress().show();
    } else {
        main_ui().detail_info_progress().hide();
    }

    if let Some(ref subtitle) = status.subtitle {
        main_ui().detail_info_substatus().set_text(subtitle);
        main_ui().detail_info_substatus().show();
    } else {
        main_ui().detail_info_substatus().hide();
    }

    if let Some(backup_status::Stats::Final(run_info)) = &status.stats {
        let mut message = String::new();

        if !matches!(run_info.outcome, borg::Outcome::Completed { .. }) {
            message.push_str(&run_info.outcome.to_string());
            message.push_str("\n\n");
        }

        message.push_str(&run_info.messages.clone().filter_hidden().to_string());

        main_ui().detail_info_error().set_text(&message);
        main_ui().detail_info_error().show();
    } else {
        main_ui().detail_info_error().hide();
    }

    match &status.stats {
        Some(backup_status::Stats::Final(RunInfo {
            outcome: borg::Outcome::Completed { stats },
            ..
        })) => {
            main_ui().detail_stats().show();
            main_ui().detail_path_row().hide();

            main_ui()
                .detail_original_size()
                .set_text(&glib::format_size(stats.archive.stats.original_size));
            main_ui()
                .detail_deduplicated_size()
                .set_text(&glib::format_size(stats.archive.stats.deduplicated_size));
            main_ui()
                .detail_nfiles()
                .set_text(&stats.archive.stats.nfiles.to_formatted_string(&*LC_LOCALE));
        }
        Some(backup_status::Stats::Progress(progress_archive)) => {
            main_ui().detail_stats().show();
            main_ui().detail_path_row().show();

            main_ui()
                .detail_original_size()
                .set_text(&glib::format_size(progress_archive.original_size));
            main_ui()
                .detail_deduplicated_size()
                .set_text(&glib::format_size(progress_archive.deduplicated_size));
            main_ui()
                .detail_nfiles()
                .set_text(&progress_archive.nfiles.to_formatted_string(&*LC_LOCALE));

            main_ui()
                .detail_current_path()
                .set_text(&format!("/{}", progress_archive.path));
            main_ui()
                .detail_current_path()
                .set_tooltip_text(Some(&format!("/{}", progress_archive.path)));
        }
        _ => {
            main_ui().detail_stats().hide();
        }
    }
}
