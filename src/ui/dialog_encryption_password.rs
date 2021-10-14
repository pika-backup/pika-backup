use gtk::prelude::*;

use crate::ui;
use zeroize::Zeroizing;

use crate::ui::prelude::*;

pub struct Ask {
    pre_select_store: bool,
    purpose: Option<String>,
}

impl Ask {
    pub fn new() -> Self {
        Self {
            pre_select_store: true,
            purpose: None,
        }
    }

    pub fn pre_select_store(&mut self, b: bool) -> &mut Self {
        self.pre_select_store = b;
        self
    }

    pub fn purpose(&mut self, purpose: String) -> &mut Self {
        self.purpose = Some(purpose);
        self
    }

    pub async fn run(&self) -> Option<(Zeroizing<Vec<u8>>, bool)> {
        let ui = ui::builder::DialogEncryptionPassword::new();

        ui.dialog().set_transient_for(Some(&main_ui().window()));
        ui.dialog().present();

        if !self.pre_select_store {
            ui.password_forget().set_active(true);
        }

        if let Some(purpose) = &self.purpose {
            ui.purpose().set_text(purpose);
            ui.purpose().show();
        } else {
            ui.purpose().hide();
        }

        let response = ui.dialog().run_future().await;
        let password = Zeroizing::new(ui.password().text().as_bytes().to_vec());
        ui.dialog().close();
        ui.dialog().hide();

        if gtk::ResponseType::Apply == response {
            Some((password, ui.password_store().is_active()))
        } else {
            None
        }
    }
}
