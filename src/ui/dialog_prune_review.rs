use adw::prelude::*;
use async_std::prelude::*;

use crate::borg;
use crate::config;
use crate::ui;
use crate::ui::prelude::*;
use ui::builder::DialogPruneReview;

pub async fn run(config: &config::Backup) -> Result<()> {
    let ui = DialogPruneReview::new();
    let guard = QuitGuard::default();

    scopeguard::defer! {
        ui.dialog().destroy();
    }

    let (sender, mut receiver) = async_std::channel::bounded(1);

    ui.apply().connect_clicked(enclose!(
        (sender) move | _ | {
            let _ignore = sender.try_send(true);
        }
    ));

    ui.dialog().connect_close_request(enclose!(
        (sender) move | _ | {
            let _ignore = sender.try_send(false);
            glib::Propagation::Proceed

        }
    ));

    ui.dialog().set_transient_for(Some(&main_ui().window()));
    ui.dialog().present();

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
    ui.stack().set_visible_child(&ui.page_decision());

    if Some(true) == receiver.next().await {
        Ok(())
    } else {
        Err(Error::UserCanceled)
    }
}
