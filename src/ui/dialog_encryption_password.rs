use adw::prelude::*;

use crate::ui;

use crate::config;
use crate::ui::prelude::*;

pub struct Ask {
    repo: config::Repository,
    purpose: String,
    keyring_error: Option<String>,
}

impl Ask {
    pub const fn new(
        repo: config::Repository,
        purpose: String,
        keyring_error: Option<String>,
    ) -> Self {
        Self {
            repo,
            purpose,
            keyring_error,
        }
    }

    pub async fn run(&self) -> Option<config::Password> {
        let ui = ui::builder::DialogEncryptionPassword::new();

        ui.dialog().set_transient_for(Some(&main_ui().window()));

        let mut body = gettextf(
            "The operation “{}” requires the encryption password of the repository on “{}”.",
            &[&self.purpose, &self.repo.location()],
        );

        if let Some(keyring_error) = &self.keyring_error {
            body.push_str(&format!("\n\n{}", keyring_error));
        }

        ui.dialog().set_body(&body);

        ui.password().grab_focus();

        ui.dialog().present();

        let response = ui.dialog().choose_future().await;
        let password = config::Password::new(ui.password().text().to_string());

        if response == "apply" {
            Some(password)
        } else {
            None
        }
    }
}
