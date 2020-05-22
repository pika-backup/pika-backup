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
        let ui = ui::builder::EncryptionPassword::new();

        ui.dialog().show_all();
        ui.dialog().set_transient_for(Some(&main_ui().window()));

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
        let password = ui
            .password()
            .get_text()
            .map(|x| x.as_bytes().to_vec())
            .map(Zeroizing::new);
        ui.dialog().close();

        match password {
            Some(password) if gtk::ResponseType::Ok == response => {
                Some((password, ui.password_store().get_active()))
            }
            _ => None,
        }
    }
}
