use crate::config::{self, Password};
use crate::ui::prelude::*;

pub async fn password(purpose: String) -> Option<config::Password> {
    crate::ui::dialog_encryption_password::Ask::new()
        .purpose(purpose)
        .run()
        .await
}

pub fn store_password(config: &config::Backup, password: &Password) -> Result<()> {
    debug!("Storing new password at secret service");
    set_password(config, password).err_to_msg(gettext("Failed to store password."))?;

    Ok(())
}

pub fn remove_password(config: &config::Backup) -> Result<()> {
    debug!("Removing password from secret service");
    delete_passwords(config).err_to_msg(gettext(
        "Failed to remove potentially remaining passwords from key storage.",
    ))?;

    Ok(())
}

fn set_password(
    config: &config::Backup,
    password: &Password,
) -> std::result::Result<(), secret_service::Error> {
    secret_service::SecretService::new(secret_service::EncryptionType::Dh)?
        .get_default_collection()?
        .create_item(
            // Translators: This is the description for entries in the password database.
            &gettext("Pika Backup Password"),
            [
                ("backup_id", config.id.as_str()),
                ("program", env!("CARGO_PKG_NAME")),
            ]
            .iter()
            .cloned()
            .collect(),
            password,
            true,
            "text/plain",
        )?;

    Ok(())
}

fn delete_passwords(config: &config::Backup) -> std::result::Result<(), secret_service::Error> {
    secret_service::SecretService::new(secret_service::EncryptionType::Dh)?
        .get_default_collection()?
        .search_items(
            [
                ("backup_id", config.id.as_str()),
                ("program", env!("CARGO_PKG_NAME")),
            ]
            .iter()
            .cloned()
            .collect(),
        )?
        .iter()
        .try_for_each(|item| item.delete())
}
