use adw::prelude::*;
use adw::subclass::prelude::*;
use chrono::prelude::*;

use super::frequency;
use super::imp;
use super::prune_preset;
use super::weekday;
use crate::config;
use crate::ui;
use crate::ui::prelude::*;

impl imp::SchedulePage {
    pub async fn show_page(&self) -> Result<()> {
        if self.obj().is_visible() {
            let backups = BACKUP_CONFIG.load();
            let config = backups.active()?;

            self.schedule_active
                .block_signal(self.schedule_active_signal_handler.get().unwrap());
            self.schedule_active
                .set_enable_expansion(config.schedule.enabled);
            self.schedule_active
                .unblock_signal(self.schedule_active_signal_handler.get().unwrap());

            self.update_status(config).await;

            match config.schedule.frequency {
                config::Frequency::Hourly => self.frequency.set_selected(0),
                config::Frequency::Daily { preferred_time } => {
                    self.frequency.set_selected(1);
                    self.preferred_hour.set_value(preferred_time.hour() as f64);
                    self.preferred_minute
                        .set_value(preferred_time.minute() as f64);
                }
                config::Frequency::Weekly { preferred_weekday } => {
                    self.frequency.set_selected(2);
                    self.preferred_weekday_row
                        .set_selected(preferred_weekday.num_days_from_monday());
                }
                config::Frequency::Monthly { preferred_day } => {
                    self.frequency.set_selected(3);
                    self.preferred_day.set_value(preferred_day as f64);
                }
            }

            // manually because signal might not have fired if already selected
            self.frequency_change().await?;

            // prune
            self.prune_save_revealer.set_reveal_child(false);

            self.prune_enabled.set_active(config.prune.enabled);
            self.prune_preset
                .set_selected(prune_preset::PrunePreset::matching(&config.prune.keep) as u32);

            self.update_prune_details(config);
        }

        Ok(())
    }

    pub async fn network_changed(&self) -> Result<()> {
        debug!("Network changed");
        if self.obj().is_visible() {
            self.update_status(BACKUP_CONFIG.load().active()?).await;
        }
        Ok(())
    }

    fn update_prune_details(&self, config: &config::Backup) {
        self.keep_hourly.set_value(config.prune.keep.hourly as f64);
        self.keep_daily.set_value(config.prune.keep.daily as f64);
        self.keep_weekly.set_value(config.prune.keep.weekly as f64);
        self.keep_monthly
            .set_value(config.prune.keep.monthly as f64);
        self.keep_yearly.set_value(config.prune.keep.yearly as f64);
    }

    pub async fn update_status(&self, config: &config::Backup) {
        let status = super::status::Status::new(config).await;

        self.status_row
            .set_title(&glib::markup_escape_text(&status.main.title()));
        self.status_row.set_subtitle(&glib::markup_escape_text(
            &status.main.subtitle().unwrap_or_default(),
        ));
        self.status_row.set_icon_name(status.main.icon_name());
        self.status_row.set_level(status.main.level());

        while let Some(row) = self.status_list.row_at_index(1) {
            self.status_list.remove(&row);
        }

        for problem in status.problems {
            self.status_list.append(&problem);
        }
    }

    fn frequency(&self) -> Result<config::Frequency> {
        if let Some(frequency) = self
            .frequency
            .selected_item()
            .and_then(|x| x.downcast::<frequency::FrequencyObject>().ok())
        {
            Ok(match frequency.frequency() {
                config::Frequency::Hourly => config::Frequency::Hourly,
                config::Frequency::Daily { .. } => config::Frequency::Daily {
                    preferred_time: chrono::NaiveTime::from_hms_opt(
                        self.preferred_hour.value() as u32,
                        self.preferred_minute.value() as u32,
                        0,
                    )
                    .ok_or_else(|| Message::short(gettext("Invalid time format.")))?,
                },
                config::Frequency::Weekly { .. } => config::Frequency::Weekly {
                    preferred_weekday: self
                        .preferred_weekday_row
                        .selected_cast()
                        .as_ref()
                        .map(weekday::WeekdayObject::weekday)
                        .ok_or_else(|| Message::short(gettext("Invalid weekday.")))?,
                },
                config::Frequency::Monthly { .. } => config::Frequency::Monthly {
                    preferred_day: self.preferred_day.value() as u8,
                },
            })
        } else {
            Err(Message::short(gettext("No frequency selected.")).into())
        }
    }

    pub async fn frequency_change(&self) -> Result<()> {
        let frequency = self.frequency()?;
        self.preferred_time_row.set_visible(false);
        self.preferred_weekday_row.set_visible(false);
        self.preferred_day.set_visible(false);

        match frequency {
            config::Frequency::Hourly => {}
            config::Frequency::Daily { .. } => {
                self.preferred_time_row.set_visible(true);
            }
            config::Frequency::Weekly { .. } => {
                self.preferred_weekday_row.set_visible(true);
            }
            config::Frequency::Monthly { .. } => {
                self.preferred_day.set_visible(true);
            }
        }

        // Reset the frequency values if the config actually changed
        // TODO: This would be much nicer if we refactored this as a GObject
        let backups = BACKUP_CONFIG.load();
        let config = backups.active()?;
        if config.schedule.frequency != frequency {
            self.preferred_hour
                .set_value(glib::random_int_range(1, 24) as f64);
            self.preferred_minute.set_value(0.);

            self.preferred_weekday_row
                .set_selected(glib::random_int_range(0, 7) as u32);

            self.preferred_day
                .set_value(glib::random_int_range(1, 32) as f64);

            BACKUP_CONFIG.try_update(enclose!(
                (frequency) | config | {
                    config.active_mut()?.schedule.frequency = frequency;
                    Ok(())
                }
            ))?;

            self.update_status(BACKUP_CONFIG.load().active()?).await;
        }

        Ok(())
    }

    pub async fn preferred_time_close(&self) -> Result<()> {
        BACKUP_CONFIG.try_update(|config| {
            config.active_mut()?.schedule.frequency = self.frequency()?;
            Ok(())
        })?;

        self.update_status(BACKUP_CONFIG.load().active()?).await;
        Ok(())
    }

    pub fn preferred_time_change(&self, button: &gtk::SpinButton) -> glib::Propagation {
        self.preferred_time_button.set_label(&format!(
            "{:02}\u{2009}:\u{2009}{:02}",
            self.preferred_hour.value(),
            self.preferred_minute.value()
        ));

        button.set_text(&format!("{:02}", button.value()));

        glib::Propagation::Stop
    }

    pub async fn preferred_weekday_change(&self) -> Result<()> {
        BACKUP_CONFIG.try_update(|config| {
            config.active_mut()?.schedule.frequency = self.frequency()?;
            Ok(())
        })?;

        self.update_status(BACKUP_CONFIG.load().active()?).await;
        Ok(())
    }

    pub async fn preferred_day_change(&self) -> Result<()> {
        BACKUP_CONFIG.try_update(|config| {
            config.active_mut()?.schedule.frequency = self.frequency()?;
            Ok(())
        })?;

        self.update_status(BACKUP_CONFIG.load().active()?).await;
        Ok(())
    }

    /// Scheduled backups activated/deactivated
    pub async fn active_change(&self) -> Result<()> {
        let active = self.schedule_active.enables_expansion();

        if !active
            && ui::utils::confirmation_dialog(
                &gettext("Disable backup schedule?"),
                &gettext("Backups will no longer run on a schedule"),
                &gettext("_Keep Schedule"),
                &gettext("_Disable Schedule"),
            )
            .await
            .is_err()
        {
            self.schedule_active.set_enable_expansion(true);
        }

        BACKUP_CONFIG.try_update(|config| {
            config.active_mut()?.schedule.enabled = self.schedule_active.enables_expansion();
            Ok(())
        })?;

        self.update_status(BACKUP_CONFIG.load().active()?).await;

        if active {
            ui::utils::background_permission().await?;
        }

        Ok(())
    }

    pub async fn prune_save(&self) -> Result<()> {
        let mut config = BACKUP_CONFIG.load().active()?.clone();
        config.prune.keep = self.keep();

        ui::widget::dialog::PruneReviewDialog::review(&self.obj().app_window(), &config).await?;

        self.prune_write_changes().await?;

        self.prune_save_revealer.set_reveal_child(false);

        Ok(())
    }

    pub async fn prune_enabled(&self) -> Result<()> {
        let unsafe_changes = self.prune_pending_unsafe_changes()?;

        self.prune_save_revealer.set_reveal_child(unsafe_changes);

        if !unsafe_changes {
            self.prune_write_changes().await?;
        }

        Ok(())
    }

    pub async fn prune_preset_change(&self) -> Result<()> {
        if let Some(preset) = self
            .prune_preset
            .selected_item()
            .and_then(|x| x.downcast::<prune_preset::PrunePresetObject>().ok())
        {
            if let Some(keep) = preset.preset().keep() {
                let mut config = BACKUP_CONFIG.load().active()?.clone();
                config.prune.keep = keep;
                self.update_prune_details(&config);
            } else {
                self.prune_detail.set_expanded(true);
            }

            Ok(())
        } else {
            Err(Message::short(gettext("No preset selected.")).into())
        }
    }

    pub async fn keep_change(&self) -> Result<()> {
        self.prune_preset
            .set_selected(prune_preset::PrunePreset::matching(&self.keep()) as u32);

        let unsafe_changes = self.prune_pending_unsafe_changes()?;
        self.prune_save_revealer.set_reveal_child(unsafe_changes);

        if !unsafe_changes {
            self.prune_write_changes().await?;
        }

        Ok(())
    }

    fn prune_pending_unsafe_changes(&self) -> Result<bool> {
        let configs = BACKUP_CONFIG.load();
        let current_config = configs.active()?;

        Ok(
            // true if pruning enabled
            (self.prune_enabled.is_active()
            && current_config.prune.enabled != self.prune_enabled.is_active()) ||
            // true if keeping less archives
            (!self.keep().is_greater_eq_everywhere(&current_config.prune.keep)
                && self.prune_enabled.is_active()),
        )
    }

    async fn prune_write_changes(&self) -> Result<()> {
        BACKUP_CONFIG.try_update(|configs| {
            let config = configs.active_mut()?;

            config.prune.enabled = self.prune_enabled.is_active();
            config.prune.keep = self.keep();

            Ok(())
        })
    }

    fn keep(&self) -> config::Keep {
        config::Keep {
            hourly: self.keep_hourly.value() as u32,
            daily: self.keep_daily.value() as u32,
            weekly: self.keep_weekly.value() as u32,
            monthly: self.keep_monthly.value() as u32,
            yearly: self.keep_yearly.value() as u32,
        }
    }
}
