use adw::prelude::*;

use crate::borg;
use crate::config;
use crate::ui;
use crate::ui::prelude::*;
use ui::builder::DialogPrune;

pub async fn run(config: &config::Backup) -> Result<()> {
    let ui = DialogPrune::new();

    let result = show(config, &ui).await;
    if result.is_err() {
        ui.dialog().destroy();
    }
    result
}

async fn show(config: &config::Backup, ui: &DialogPrune) -> Result<()> {
    ui.dialog().set_transient_for(Some(&main_ui().window()));
    ui.dialog().present();

    let guard = QuitGuard::default();
    let prune_info = ui::utils::borg::exec(
        borg::Command::<borg::task::PruneInfo>::new(config.clone()),
        &guard,
    )
    .await
    .into_message(gettext(
        "Failed to determine how many archives would be deleted",
    ))?;

    let list_all = ui::utils::borg::exec(
        borg::Command::<borg::task::List>::new(config.clone()),
        &guard,
    )
    .await
    .into_message("List Archives")?;

    let num_untouched_archives = list_all.len() - prune_info.prune - prune_info.keep;

    ui.prune().set_label(&prune_info.prune.to_string());
    ui.keep().set_label(&prune_info.keep.to_string());
    ui.untouched()
        .set_label(&num_untouched_archives.to_string());

    if prune_info.prune == 0 {
        ui.delete().set_visible(false);
        ui.cancel().set_label(&gettext("Close"));
    }

    ui.stack().set_visible_child(&ui.page_decision());

    ui.delete()
        .connect_clicked(clone!(@weak ui, @strong config =>
           move |_|  Handler::new().error_transient_for(ui.dialog()).spawn(enclose!((config) async move {
               let result = delete(ui.clone(), config.clone()).await;
               ui.dialog().destroy();
               result
           }))
        ));

    // ensure lifetime until window closes
    let mutex = std::sync::Mutex::new(Some(ui.clone()));
    ui.dialog().connect_close_request(move |_| {
        *mutex.lock().unwrap() = None;
        glib::Propagation::Proceed
    });

    ui.dialog().connect_destroy(|_| {
        debug!("Destroy dialog");
    });

    Ok(())
}

async fn delete(ui: DialogPrune, config: config::Backup) -> Result<()> {
    ui.dialog().destroy();

    let guard = QuitGuard::default();
    let result = ui::utils::borg::exec(
        borg::Command::<borg::task::Prune>::new(config.clone()),
        &guard,
    )
    .await;

    if !result.is_borg_err_user_aborted() {
        result.into_message(gettext("Delete old Archives"))?;
    }

    let result = ui::utils::borg::exec(
        borg::Command::<borg::task::Compact>::new(config.clone()),
        &guard,
    )
    .await;

    if !result.is_borg_err_user_aborted() {
        result.into_message(gettext("Reclaim Free Space"))?;
    }

    let _ignore = ui::page_archives::cache::refresh_archives(config.clone(), None).await;
    let _ignore = ui::utils::df::lookup_and_cache(&config).await;

    Ok(())
}
