use adw::prelude::*;
use chrono::prelude::*;

use super::frequency;
use super::init;
use super::weekday;
use crate::config;
use crate::ui;
use crate::ui::prelude::*;

pub async fn show_page() -> Result<()> {
    if main_ui().detail_stack().visible_child()
        == Some(main_ui().page_schedule().upcast::<gtk::Widget>())
    {
        let backups = BACKUP_CONFIG.load();
        let config = backups.active()?;

        main_ui()
            .schedule_active()
            .block_signal(&init::SCHEDULE_ACTIVE_SIGNAL_HANDLER);
        main_ui()
            .schedule_active()
            .set_enable_expansion(config.schedule.enabled);
        main_ui()
            .schedule_active()
            .unblock_signal(&init::SCHEDULE_ACTIVE_SIGNAL_HANDLER);

        update_status(config);

        match config.schedule.frequency {
            config::Frequency::Hourly => main_ui().schedule_frequency().set_selected(0),
            config::Frequency::Daily { preferred_time } => {
                main_ui().schedule_frequency().set_selected(1);
                main_ui()
                    .schedule_preferred_hour()
                    .set_value(preferred_time.hour() as f64);
                main_ui()
                    .schedule_preferred_minute()
                    .set_value(preferred_time.minute() as f64);
            }
            config::Frequency::Weekly { preferred_weekday } => {
                main_ui().schedule_frequency().set_selected(2);
                main_ui()
                    .preferred_weekday_row()
                    .set_selected(preferred_weekday.num_days_from_monday());
            }
            config::Frequency::Monthly { preferred_day } => {
                main_ui().schedule_frequency().set_selected(3);
                main_ui()
                    .schedule_preferred_day_calendar()
                    .set_day(preferred_day as i32);
            }
        }

        // manually because signal might not have fired if already selected
        frequency_change().await?;
    }

    Ok(())
}

fn update_status(config: &config::Backup) {
    let status = super::status::Status::new(config);

    main_ui().schedule_status().set_title(&status.main.title);
    main_ui()
        .schedule_status()
        .set_subtitle(&status.main.subtitle);
    main_ui()
        .schedule_status_icon()
        .set_icon_name(&status.main.icon_name);
    main_ui()
        .schedule_status_icon()
        .set_level(status.main.level);

    while let Some(row) = main_ui().schedule_status_list().row_at_index(1) {
        main_ui().schedule_status_list().remove(&row);
    }

    for problem in status.problems {
        main_ui()
            .schedule_status_list()
            .append(&problem.action_row());
    }
}

fn frequency() -> Result<config::Frequency> {
    if let Some(frequency) = main_ui()
        .schedule_frequency()
        .selected_item()
        .and_then(|x| x.downcast::<frequency::FrequencyObject>().ok())
    {
        Ok(match frequency.frequency() {
            config::Frequency::Hourly => config::Frequency::Hourly,
            config::Frequency::Daily { .. } => config::Frequency::Daily {
                preferred_time: chrono::NaiveTime::from_hms_opt(
                    main_ui().schedule_preferred_hour().value() as u32,
                    main_ui().schedule_preferred_minute().value() as u32,
                    0,
                )
                .ok_or_else(|| Message::short(gettext("Invalid time format.")))?,
            },
            config::Frequency::Weekly { .. } => config::Frequency::Weekly {
                preferred_weekday: main_ui()
                    .preferred_weekday_row()
                    .selected_cast()
                    .as_ref()
                    .map(weekday::WeekdayObject::weekday)
                    .ok_or_else(|| Message::short(gettext("Invalid weekday.")))?,
            },
            config::Frequency::Monthly { .. } => config::Frequency::Monthly {
                preferred_day: main_ui().schedule_preferred_day_calendar().day() as u8,
            },
        })
    } else {
        Err(Message::short(gettext("No frequency selected.")).into())
    }
}

pub async fn frequency_change() -> Result<()> {
    let frequency = frequency()?;

    match frequency {
        config::Frequency::Hourly => {
            main_ui().preferred_time_row().hide();
            main_ui().preferred_weekday_row().hide();
            main_ui().preferred_day_row().hide();
        }
        config::Frequency::Daily { .. } => {
            main_ui().preferred_time_row().show();

            main_ui().preferred_weekday_row().hide();
            main_ui().preferred_day_row().hide();
        }
        config::Frequency::Weekly { .. } => {
            main_ui().preferred_weekday_row().show();

            main_ui().preferred_time_row().hide();
            main_ui().preferred_day_row().hide();
        }
        config::Frequency::Monthly { .. } => {
            main_ui().preferred_day_row().show();

            main_ui().preferred_time_row().hide();
            main_ui().preferred_weekday_row().hide();
        }
    }

    BACKUP_CONFIG.update_result(enclose!(
        (frequency) | config | {
            config.active_mut()?.schedule.frequency = frequency;
            Ok(())
        }
    ))?;

    update_status(BACKUP_CONFIG.load().active()?);
    ui::write_config()
}

pub async fn preferred_time_close() -> Result<()> {
    BACKUP_CONFIG.update_result(|config| {
        config.active_mut()?.schedule.frequency = frequency()?;
        Ok(())
    })?;

    update_status(BACKUP_CONFIG.load().active()?);
    ui::write_config()
}

pub fn preferred_time_change(button: &gtk::SpinButton) -> gtk::Inhibit {
    main_ui()
        .schedule_preferred_time_button()
        .set_label(&format!(
            "{:02}\u{2009}:\u{2009}{:02}",
            main_ui().schedule_preferred_hour().value(),
            main_ui().schedule_preferred_minute().value()
        ));

    button.set_text(&format!("{:02}", button.value()));

    gtk::Inhibit(true)
}

pub async fn preferred_weekday_change() -> Result<()> {
    BACKUP_CONFIG.update_result(|config| {
        config.active_mut()?.schedule.frequency = frequency()?;
        Ok(())
    })?;

    update_status(BACKUP_CONFIG.load().active()?);
    ui::write_config()
}

pub async fn preferred_day_change() -> Result<()> {
    main_ui().schedule_preferred_day_popover().popdown();
    main_ui().schedule_preferred_day().set_label(&format!(
        "{}st",
        main_ui().schedule_preferred_day_calendar().day(),
    ));

    BACKUP_CONFIG.update_result(|config| {
        config.active_mut()?.schedule.frequency = frequency()?;
        Ok(())
    })?;

    update_status(BACKUP_CONFIG.load().active()?);
    ui::write_config()
}

/// Scheduled backups activated/deactivated
pub async fn active_change() -> Result<()> {
    if !main_ui().schedule_active().enables_expansion()
        && ui::utils::confirmation_dialog(
            &gettext("Disable backup schedule."),
            &gettext("No longer remind of backups based on a schedule."),
            &gettext("Keep Schedule"),
            &gettext("Disable Schedule"),
        )
        .await
        .is_err()
    {
        main_ui().schedule_active().set_enable_expansion(true);
    }

    BACKUP_CONFIG.update_result(|config| {
        config.active_mut()?.schedule.enabled = main_ui().schedule_active().enables_expansion();
        Ok(())
    })?;

    update_status(BACKUP_CONFIG.load().active()?);
    ui::write_config()
}
