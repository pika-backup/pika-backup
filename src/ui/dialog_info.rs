use gtk::prelude::*;

use num_format::ToFormattedString;

use crate::history::*;
use crate::ui::backup_status;
use crate::ui::prelude::*;

pub fn init() {
    main_ui().detail_dialog_vbox().set_border_width(0);

    main_ui()
        .detail_running_backup_info()
        .connect_delete_event(|x, _| WidgetExtManual::hide_on_delete(x));

    glib::timeout_add_local(250, || {
        refresh_status();
        Continue(true)
    });
}

fn is_visible() -> bool {
    main_ui().detail_running_backup_info().get_visible()
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
        main_ui().detail_info_substatus().set_text(&subtitle);
        main_ui().detail_info_substatus().show();
    } else {
        main_ui().detail_info_substatus().hide();
    }

    if let Some(backup_status::Stats::Final(RunInfo {
        result: Err(err), ..
    })) = &status.stats
    {
        main_ui().detail_info_error().set_text(&format!("{}", err));
        main_ui().detail_info_error().show();
    } else {
        main_ui().detail_info_error().hide();
    }

    match &status.stats {
        Some(backup_status::Stats::Final(RunInfo {
            result: Ok(stats), ..
        }))
        | Some(backup_status::Stats::Final(RunInfo {
            result: Err(RunError::WithLevel {
                stats: Some(stats), ..
            }),
            ..
        })) => {
            main_ui().detail_stats().show();
            main_ui().detail_path_row().hide();

            main_ui()
                .detail_original_size()
                .set_text(&glib::format_size(stats.archive.stats.original_size).unwrap());
            main_ui()
                .detail_deduplicated_size()
                .set_text(&glib::format_size(stats.archive.stats.deduplicated_size).unwrap());
            main_ui()
                .detail_nfiles()
                .set_text(&stats.archive.stats.nfiles.to_formatted_string(&*LC_LOCALE));
        }
        Some(backup_status::Stats::Progress(progress_archive)) => {
            main_ui().detail_stats().show();
            main_ui().detail_path_row().show();

            main_ui()
                .detail_original_size()
                .set_text(&glib::format_size(progress_archive.original_size).unwrap());
            main_ui()
                .detail_deduplicated_size()
                .set_text(&glib::format_size(progress_archive.deduplicated_size).unwrap());
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
