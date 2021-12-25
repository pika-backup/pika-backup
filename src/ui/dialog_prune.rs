use crate::borg;
use crate::ui;
use crate::ui::prelude::*;
use adw::prelude::*;

pub async fn run(prune_info: &borg::PruneInfo) -> bool {
    let ui = ui::builder::DialogPrune::new();

    ui.dialog().set_transient_for(Some(&main_ui().window()));
    ui.prune().set_label(&prune_info.prune.to_string());
    ui.keep().set_label(&prune_info.keep.to_string());

    let result = ui.dialog().run_future().await == gtk::ResponseType::Apply;

    ui.dialog().destroy();

    result
}
