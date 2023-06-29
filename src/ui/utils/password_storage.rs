use crate::config::{self, Password};
use crate::ui::prelude::*;
use std::collections::HashMap;

pub async fn password_dialog(
    repo: config::Repository,
    purpose: String,
    keyring_error: Option<String>,
) -> Option<config::Password> {
    crate::ui::dialog_encryption_password::Ask::new(repo, purpose, keyring_error)
        .run()
        .await
}

pub async fn store_password(config: &config::Backup, password: &Password) -> Result<()> {
    debug!("Storing new password at secret service");
    set_password(config, password)
        .await
        .map_err(|err| Message::from_secret_service(gettext("Failed to Store Password"), err))?;

    Ok(())
}

pub async fn remove_password(config: &config::Backup, remove_all: bool) -> Result<()> {
    // check if other configs using this repo exist
    if !remove_all
        && BACKUP_CONFIG
            .load()
            .iter()
            .any(|x| x.id != config.id && x.repo_id == config.repo_id)
    {
        debug!("Not removing password because other configs need it");
    } else {
        debug!("Removing password from secret service");
        delete_passwords(config).await.map_err(|err| {
            Message::from_secret_service(
                gettext("Failed to remove potentially remaining passwords from key storage."),
                err,
            )
        })?;
    }

    Ok(())
}

async fn set_password(
    config: &config::Backup,
    password: &Password,
) -> std::result::Result<(), oo7::Error> {
    debug!("Starting to store password");
    let keyring = oo7::Keyring::new().await?;

    keyring
        .create_item(
            // Translators: This is the description for entries in the password database.
            &gettextf("Pika Backup “{}”", &[&config.repo.location()]),
            HashMap::from([("repo-id", config.repo_id.as_str())]),
            password.as_bytes(),
            true,
        )
        .await?;

    debug!("Storing password returned");

    Ok(())
}

async fn delete_passwords(config: &config::Backup) -> std::result::Result<(), oo7::Error> {
    debug!("Starting to clear passwords");

    let keyring = oo7::Keyring::new().await?;
    keyring
        .delete(HashMap::from([("repo-id", config.repo_id.as_str())]))
        .await?;

    debug!("Clearing password returned");
    Ok(())
}
