use adw::prelude::*;
use ui::prelude::*;

use crate::config;
use crate::ui;

pub async fn run(config: &config::Backup) -> Result<()> {
    let ui = ui::builder::DialogArchivePrefix::new();

    ui.archive_prefix()
        .set_text(&config.archive_prefix.to_string());
    ui.archive_prefix().grab_focus();

    ui.dialog().set_transient_for(Some(&main_ui().window()));

    let response = ui.dialog().run_future().await;

    if response == gtk::ResponseType::Apply {
        let config_id = config.id.clone();
        let new_prefix = ui.archive_prefix().text();

        BACKUP_CONFIG.update_result(enclose!(
            (config_id, new_prefix) | config | {
                config.get_result_mut(&config_id)?.archive_prefix =
                    config::ArchivePrefix::new(&new_prefix);
                Ok(())
            }
        ))?;

        ui::write_config()?;
    }

    ui.dialog().close();

    Ok(())
}
