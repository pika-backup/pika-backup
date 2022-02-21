use gtk::prelude::*;

use crate::ui;

use crate::config;
use crate::ui::prelude::*;

pub struct Ask {
    purpose: Option<String>,
}

impl Ask {
    pub fn new() -> Self {
        Self { purpose: None }
    }

    pub fn purpose(&mut self, purpose: String) -> &mut Self {
        self.purpose = Some(purpose);
        self
    }

    pub async fn run(&self) -> Option<config::Password> {
        let ui = ui::builder::DialogEncryptionPassword::new();

        ui.dialog().set_transient_for(Some(&main_ui().window()));
        ui.dialog().present();

        if let Some(purpose) = &self.purpose {
            ui.purpose().set_text(purpose);
            ui.purpose().show();
        } else {
            ui.purpose().hide();
        }

        let response = ui.dialog().run_future().await;
        let password = config::Password::new(ui.password().text().to_string());
        ui.dialog().close();
        ui.dialog().hide();

        if gtk::ResponseType::Apply == response {
            Some(password)
        } else {
            None
        }
    }
}
