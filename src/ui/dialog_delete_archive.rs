use adw::prelude::*;

use crate::borg;
use crate::config;
use crate::ui;
use crate::ui::prelude::*;
use ui::builder::DialogDeleteArchive;

pub async fn run(config: &config::Backup, archive_name: &str, archive_date: &str) -> Result<()> {
    let ui = DialogDeleteArchive::new();

    let result = show(config, archive_name, archive_date, &ui).await;
    if result.is_err() {
        ui.dialog().destroy();
    }
    result
}

async fn show(
    config: &config::Backup,
    archive_name: &str,
    archive_date: &str,
    ui: &DialogDeleteArchive,
) -> Result<()> {
    ui.dialog().set_transient_for(Some(&main_ui().window()));
    ui.dialog().present();

    ui.leaflet().set_visible_child(&ui.page_decision());

    let archive_name = archive_name.to_string();
    ui.name().set_label(&archive_name);

    let archive_date = archive_date.to_string();
    ui.date().set_label(&archive_date);

    ui.delete()
        .connect_clicked(clone!(@weak ui, @strong config, @strong archive_name =>
           move |_|  Handler::new().error_transient_for(ui.dialog()).spawn(enclose!((config, archive_name) async move {
               let result = delete(ui.clone(), config.clone(), &archive_name.clone()).await;
               ui.dialog().destroy();
               result
           }))
        ));

    // ensure lifetime until window closes
    let mutex = std::sync::Mutex::new(Some(ui.clone()));
    ui.dialog().connect_close_request(move |_| {
        *mutex.lock().unwrap() = None;
        gtk::Inhibit(false)
    });

    ui.dialog().connect_destroy(|_| {
        debug!("Destroy dialog");
    });

    Ok(())
}

async fn delete(ui: DialogDeleteArchive, config: config::Backup, archive_name: &str) -> Result<()> {
    ui.dialog().destroy();

    let guard = QuitGuard::default();
    let archive_name = Some(archive_name.to_string());

    let mut command = borg::Command::<borg::task::Delete>::new(config.clone());
    command.task.set_archive_name(archive_name);
    let result = ui::utils::borg::exec(command, &guard).await;

    result.into_message(gettext("Delete Archive Failed"))?;

    ui::utils::borg::exec(
        borg::Command::<borg::task::Compact>::new(config.clone()),
        &guard,
    )
    .await
    .into_message("Reclaiming Free Space Failed")?;

    let _ = ui::page_archives::cache::refresh_archives(config, None).await;

    Ok(())
}
