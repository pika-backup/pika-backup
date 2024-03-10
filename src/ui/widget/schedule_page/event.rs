use adw::prelude::*;
use chrono::prelude::*;

use super::frequency;
use super::init;
use super::prune_preset;
use super::weekday;
use crate::config;
use crate::ui;
use crate::ui::prelude::*;

pub async fn show_page() -> Result<()> {
    if super::is_visible() {
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

        update_status(config).await;

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
                    .schedule_preferred_day()
                    .set_value(preferred_day as f64);
            }
        }

        // manually because signal might not have fired if already selected
        frequency_change().await?;

        // prune
        main_ui().prune_save_revealer().set_reveal_child(false);

        main_ui().prune_enabled().set_active(config.prune.enabled);
        main_ui()
            .prune_preset()
            .set_selected(prune_preset::PrunePreset::matching(&config.prune.keep) as u32);

        update_prune_details(config);
    }

    Ok(())
}

pub async fn network_changed() -> Result<()> {
    debug!("Network changed");
    if super::is_visible() {
        update_status(BACKUP_CONFIG.load().active()?).await;
    }
    Ok(())
}

fn update_prune_details(config: &config::Backup) {
    main_ui()
        .schedule_keep_hourly()
        .set_value(config.prune.keep.hourly as f64);
    main_ui()
        .schedule_keep_daily()
        .set_value(config.prune.keep.daily as f64);
    main_ui()
        .schedule_keep_weekly()
        .set_value(config.prune.keep.weekly as f64);
    main_ui()
        .schedule_keep_monthly()
        .set_value(config.prune.keep.monthly as f64);
    main_ui()
        .schedule_keep_yearly()
        .set_value(config.prune.keep.yearly as f64);
}

pub async fn update_status(config: &config::Backup) {
    let status = super::status::Status::new(config).await;

    main_ui()
        .schedule_status()
        .set_title(&glib::markup_escape_text(&status.main.title()));
    main_ui()
        .schedule_status()
        .set_subtitle(&glib::markup_escape_text(
            &status.main.subtitle().unwrap_or_default(),
        ));
    main_ui()
        .schedule_status()
        .set_icon_name(status.main.icon_name());
    main_ui().schedule_status().set_level(status.main.level());

    while let Some(row) = main_ui().schedule_status_list().row_at_index(1) {
        main_ui().schedule_status_list().remove(&row);
    }

    for problem in status.problems {
        main_ui().schedule_status_list().append(&problem);
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
                preferred_day: main_ui().schedule_preferred_day().value() as u8,
            },
        })
    } else {
        Err(Message::short(gettext("No frequency selected.")).into())
    }
}

pub async fn frequency_change() -> Result<()> {
    let frequency = frequency()?;
    main_ui().preferred_time_row().set_visible(false);
    main_ui().preferred_weekday_row().set_visible(false);
    main_ui().schedule_preferred_day().set_visible(false);

    match frequency {
        config::Frequency::Hourly => {}
        config::Frequency::Daily { .. } => {
            main_ui().preferred_time_row().set_visible(true);
        }
        config::Frequency::Weekly { .. } => {
            main_ui().preferred_weekday_row().set_visible(true);
        }
        config::Frequency::Monthly { .. } => {
            main_ui().schedule_preferred_day().set_visible(true);
        }
    }

    // Reset the frequency values if the config actually changed
    // TODO: This would be much nicer if we refactored this as a GObject
    let backups = BACKUP_CONFIG.load();
    let config = backups.active()?;
    if config.schedule.frequency != frequency {
        main_ui()
            .schedule_preferred_hour()
            .set_value(glib::random_int_range(1, 24) as f64);
        main_ui().schedule_preferred_minute().set_value(0.);

        main_ui()
            .preferred_weekday_row()
            .set_selected(glib::random_int_range(0, 7) as u32);

        main_ui()
            .schedule_preferred_day()
            .set_value(glib::random_int_range(1, 32) as f64);

        BACKUP_CONFIG.try_update(enclose!(
            (frequency) | config | {
                config.active_mut()?.schedule.frequency = frequency;
                Ok(())
            }
        ))?;

        update_status(BACKUP_CONFIG.load().active()?).await;
    }

    Ok(())
}

pub async fn preferred_time_close() -> Result<()> {
    BACKUP_CONFIG.try_update(|config| {
        config.active_mut()?.schedule.frequency = frequency()?;
        Ok(())
    })?;

    update_status(BACKUP_CONFIG.load().active()?).await;
    Ok(())
}

pub fn preferred_time_change(button: &gtk::SpinButton) -> glib::Propagation {
    main_ui()
        .schedule_preferred_time_button()
        .set_label(&format!(
            "{:02}\u{2009}:\u{2009}{:02}",
            main_ui().schedule_preferred_hour().value(),
            main_ui().schedule_preferred_minute().value()
        ));

    button.set_text(&format!("{:02}", button.value()));

    glib::Propagation::Stop
}

pub async fn preferred_weekday_change() -> Result<()> {
    BACKUP_CONFIG.try_update(|config| {
        config.active_mut()?.schedule.frequency = frequency()?;
        Ok(())
    })?;

    update_status(BACKUP_CONFIG.load().active()?).await;
    Ok(())
}

pub async fn preferred_day_change() -> Result<()> {
    BACKUP_CONFIG.try_update(|config| {
        config.active_mut()?.schedule.frequency = frequency()?;
        Ok(())
    })?;

    update_status(BACKUP_CONFIG.load().active()?).await;
    Ok(())
}

/// Scheduled backups activated/deactivated
pub async fn active_change() -> Result<()> {
    let active = main_ui().schedule_active().enables_expansion();

    if !active
        && ui::utils::confirmation_dialog(
            &gettext("Disable backup schedule?"),
            &gettext("No longer remind of backups based on a schedule."),
            &gettext("Keep Schedule"),
            &gettext("Disable Schedule"),
        )
        .await
        .is_err()
    {
        main_ui().schedule_active().set_enable_expansion(true);
    }

    BACKUP_CONFIG.try_update(|config| {
        config.active_mut()?.schedule.enabled = main_ui().schedule_active().enables_expansion();
        Ok(())
    })?;

    update_status(BACKUP_CONFIG.load().active()?).await;

    if active {
        ui::utils::background_permission().await?;
    }

    Ok(())
}

pub async fn prune_save() -> Result<()> {
    let mut config = BACKUP_CONFIG.load().active()?.clone();
    config.prune.keep = keep();

    ui::dialog_prune_review::run(&config).await?;

    prune_write_changes().await?;

    main_ui().prune_save_revealer().set_reveal_child(false);

    Ok(())
}

pub async fn prune_enabled() -> Result<()> {
    let unsafe_changes = prune_pending_unsafe_changes()?;

    main_ui()
        .prune_save_revealer()
        .set_reveal_child(unsafe_changes);

    if !unsafe_changes {
        prune_write_changes().await?;
    }

    Ok(())
}

pub async fn prune_preset_change() -> Result<()> {
    if let Some(preset) = main_ui()
        .prune_preset()
        .selected_item()
        .and_then(|x| x.downcast::<prune_preset::PrunePresetObject>().ok())
    {
        if let Some(keep) = preset.preset().keep() {
            let mut config = BACKUP_CONFIG.load().active()?.clone();
            config.prune.keep = keep;
            update_prune_details(&config);
        } else {
            main_ui().prune_detail().set_expanded(true);
        }

        Ok(())
    } else {
        Err(Message::short(gettext("No preset selected.")).into())
    }
}

pub async fn keep_change() -> Result<()> {
    main_ui()
        .prune_preset()
        .set_selected(prune_preset::PrunePreset::matching(&keep()) as u32);

    let unsafe_changes = prune_pending_unsafe_changes()?;
    main_ui()
        .prune_save_revealer()
        .set_reveal_child(unsafe_changes);

    if !unsafe_changes {
        prune_write_changes().await?;
    }

    Ok(())
}

fn prune_pending_unsafe_changes() -> Result<bool> {
    let configs = BACKUP_CONFIG.load();
    let current_config = configs.active()?;

    Ok(
        // true if pruning enabled
        (main_ui().prune_enabled().is_active()
            && current_config.prune.enabled != main_ui().prune_enabled().is_active()) ||
            // true if keeping less archives
            (!keep().is_greater_eq_everywhere(&current_config.prune.keep)
                && main_ui().prune_enabled().is_active()),
    )
}

async fn prune_write_changes() -> Result<()> {
    BACKUP_CONFIG.try_update(|configs| {
        let config = configs.active_mut()?;

        config.prune.enabled = main_ui().prune_enabled().is_active();
        config.prune.keep = keep();

        Ok(())
    })
}

fn keep() -> config::Keep {
    config::Keep {
        hourly: main_ui().schedule_keep_hourly().value() as u32,
        daily: main_ui().schedule_keep_daily().value() as u32,
        weekly: main_ui().schedule_keep_weekly().value() as u32,
        monthly: main_ui().schedule_keep_monthly().value() as u32,
        yearly: main_ui().schedule_keep_yearly().value() as u32,
    }
}
