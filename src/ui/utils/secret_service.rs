use crate::config::{self, Password};
use crate::ui::prelude::*;
use std::collections::HashMap;

pub async fn password(repo: config::Repository, purpose: String) -> Option<config::Password> {
    crate::ui::dialog_encryption_password::Ask::new(repo, purpose)
        .run()
        .await
}

pub async fn store_password(config: &config::Backup, password: &Password) -> Result<()> {
    debug!("Storing new password at secret service");
    set_password(config, password)
        .await
        .err_to_msg(gettext("Failed to Store Password"))?;

    Ok(())
}

pub async fn remove_password(config: &config::Backup) -> Result<()> {
    debug!("Removing password from secret service");
    delete_passwords(config).await.err_to_msg(gettext(
        "Failed to remove potentially remaining passwords from key storage.",
    ))?;

    Ok(())
}

async fn set_password(
    config: &config::Backup,
    password: &Password,
) -> std::result::Result<(), glib::Error> {
    debug!("Starting to store password");
    let result = libsecret::password_store_future(
        Some(&config::Password::libsecret_schema()),
        HashMap::from([("repo-id", config.repo_id.as_str())]),
        None,
        // Translators: This is the description for entries in the password database.
        &gettextf("Pika Backup “{}”", &[&config.repo.location()]),
        password.as_str(),
    )
    .await;
    debug!("Storing password returned");
    result
}

async fn delete_passwords(config: &config::Backup) -> std::result::Result<(), glib::Error> {
    debug!("Starting to clear passwords");
    let result = libsecret::password_clear_future(
        Some(&config::Password::libsecret_schema()),
        HashMap::from([("repo-id", config.repo_id.as_str())]),
    )
    .await;
    debug!("Clearing password returned");
    result
}
