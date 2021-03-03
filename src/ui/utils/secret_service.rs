use crate::config::{self, Password};
use crate::ui::prelude::*;

pub fn set_password(
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

pub fn delete_passwords(id: &ConfigId) -> std::result::Result<(), secret_service::Error> {
    secret_service::SecretService::new(secret_service::EncryptionType::Dh)?
        .get_default_collection()?
        .search_items(
            [
                ("backup_id", id.as_str()),
                ("program", env!("CARGO_PKG_NAME")),
            ]
            .iter()
            .cloned()
            .collect(),
        )?
        .iter()
        .try_for_each(|item| item.delete())
}

pub async fn get_password(pre_select_store: bool) -> Option<(config::Password, bool)> {
    crate::ui::dialog_encryption_password::Ask::new()
        .set_pre_select_store(pre_select_store)
        .run()
        .await
}

pub fn store_password(config: &config::Backup, password: &Option<(Password, bool)>) -> Result<()> {
    if let Some((password, store)) = &password {
        if *store {
            debug!("Storing new password at secret service");
            set_password(config, password).err_to_msg(gettext("Failed to store password."))?;
        } else {
            debug!("Removing password from secret service");
            delete_passwords(&config.id).err_to_msg(gettext(
                "Failed to remove potentially remaining passwords from key storage.",
            ))?;
        }
    }

    Ok(())
}
