use gtk::prelude::*;

use crate::ui;
use zeroize::Zeroizing;

use crate::ui::globals::*;

pub struct Ask {
    pre_select_store: bool,
}

impl Ask {
    pub fn new() -> Self {
        Self {
            pre_select_store: true,
        }
    }

    pub fn set_pre_select_store(&mut self, b: bool) -> &mut Self {
        self.pre_select_store = b;
        self
    }

    pub fn run(&self) -> Option<(Zeroizing<Vec<u8>>, bool)> {
        let ui = ui::builder::DialogEncryptionPassword::new();

        ui.dialog().set_transient_for(Some(&main_ui().window()));
        ui.dialog().show_all();

        if !self.pre_select_store {
            ui.password_forget().set_active(true);
        }

        let dialog = ui.dialog();
        ui.cancel()
            .connect_clicked(move |_| dialog.response(gtk::ResponseType::Cancel));
        let dialog = ui.dialog();
        ui.ok()
            .connect_clicked(move |_| dialog.response(gtk::ResponseType::Ok));

        let response = ui.dialog().run();
        let password = Zeroizing::new(ui.password().get_text().as_bytes().to_vec());
        ui.dialog().close();
        ui.dialog().hide();

        if gtk::ResponseType::Ok == response {
            Some((password, ui.password_store().get_active()))
        } else {
            None
        }
    }
}
