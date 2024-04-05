use super::imp;
use crate::borg;
use crate::borg::prelude::*;
use crate::config;
use crate::ui;
use crate::ui::prelude::*;

impl imp::SetupDialog {
    pub(super) async fn init_repo(
        &self,
        repo: crate::config::Repository,
        password: Option<crate::config::Password>,
    ) -> Result<crate::config::ConfigId> {
        let encrypted = password.is_some();

        let mut borg = borg::CommandOnlyRepo::new(repo.clone());
        if let Some(password) = &password {
            borg.set_password(password.clone());
        }

        ui::utils::borg::exec_repo_only(
            &gettext("Creating Backup Repository"),
            borg.clone(),
            |borg| borg.init(),
        )
        .await
        .into_message("Failed to Initialize Repository")?;

        // Get repo id
        let info = ui::utils::borg::exec_repo_only(
            &gettext("Getting Repository Information"),
            borg,
            |borg| borg.peek(),
        )
        .await
        .into_message("Failed to Obtain Repository Information")?;

        let config = config::Backup::new(repo.clone(), info, encrypted);

        BACKUP_CONFIG.try_update(glib::clone!(@strong config => move |s| {
            s.insert(config.clone())?;
            Ok(())
        }))?;

        if let Some(password) = &password {
            if let Err(err) = ui::utils::password_storage::store_password(&config, password).await {
                // Error when storing the password. The repository has already been created, therefore we must continue at this point.
                err.show().await;
            }
        }

        Ok(config.id)
    }
}
