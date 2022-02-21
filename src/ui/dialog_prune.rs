use adw::prelude::*;
use async_std::prelude::*;

use crate::borg;
use crate::config;
use crate::ui;
use crate::ui::error;
use crate::ui::prelude::*;
use ui::builder::DialogPrune;

pub async fn execute(config: config::Backup) -> Result<()> {
    let communication = borg::Communication::default();

    let result = ui::utils::borg::exec(
        gettext("Delete old Archives"),
        config.clone(),
        enclose!((communication) move |borg| {
            borg.prune(communication)
        }),
    )
    .await;

    if !matches!(
        result,
        Err(error::Combined::Borg(borg::Error::Aborted(
            borg::error::Abort::User
        )))
    ) {
        result.into_message(gettext("Delete old Archives"))?;
    }

    Ok(())
}

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

    let prune_info = ui::utils::borg::exec(
        gettext("Determine how many archives would be deleted"),
        config.clone(),
        |borg| borg.prune_info(),
    )
    .await
    .into_message(gettext(
        "Failed to determine how many archives would be deleted",
    ))?;

    ui.prune().set_label(&prune_info.prune.to_string());
    ui.keep().set_label(&prune_info.keep.to_string());
    ui.leaflet().set_visible_child(&ui.page_decision());

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
        gtk::Inhibit(false)
    });

    ui.dialog().connect_destroy(|_| {
        debug!("Destroy dialog");
    });

    Ok(())
}

async fn delete(ui: DialogPrune, config: config::Backup) -> Result<()> {
    ui.leaflet().set_visible_child(&ui.page_deleting());

    let communication = borg::Communication::default();

    glib::MainContext::default().spawn_local(enclose!((ui, mut communication) async move {
        let mut receiver = communication.new_receiver();
        while let Some(log) = receiver.next().await {
            let msg = log.to_string();
            if !msg.is_empty() {
               ui.deleting_status().set_description(Some(&msg));
            }
        }
        ui.deleting_status().set_description(None);
    }));

    ui.abort()
        .connect_clicked(enclose!((communication) move |_| {communication.instruction
        .store(Arc::new(borg::Instruction::Abort)
        );} ));

    let result = ui::utils::borg::exec(
        gettext("Delete old Archives"),
        config.clone(),
        enclose!((communication) move |borg| {
            borg.prune(communication)
        }),
    )
    .await;

    if !matches!(
        result,
        Err(error::Combined::Borg(borg::Error::Aborted(
            borg::error::Abort::User
        )))
    ) {
        result.into_message(gettext("Delete old Archives"))?;
    }

    Ok(())
}
