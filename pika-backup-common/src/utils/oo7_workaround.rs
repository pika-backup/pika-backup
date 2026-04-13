use std::sync::Arc;

pub async fn load_keyring() -> Result<oo7::Keyring, oo7::Error> {
    let keyring = oo7::Keyring::new().await;

    match keyring {
        Ok(keyring) => Ok(keyring),
        Err(oo7::Error::File(
            oo7::file::Error::IncorrectSecret | oo7::file::Error::PartiallyCorruptedKeyring { .. },
        )) => {
            let secret = oo7::Secret::from(
                ashpd::desktop::secret::retrieve()
                    .await
                    .map_err(oo7::file::Error::from)?,
            );

            unsafe {
                oo7::file::UnlockedKeyring::load_unchecked(
                    oo7::file::api::Keyring::default_path()?,
                    secret.clone(),
                )
            }
            .await
            .map(|x| {
                oo7::Keyring::File(Arc::new(async_lock::RwLock::new(Some(
                    oo7::file::Keyring::Unlocked(x),
                ))))
            })
            .map_err(Into::into)
        }
        Err(err) => Err(err),
    }
}
