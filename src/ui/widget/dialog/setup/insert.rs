use adw::prelude::*;

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

        Self::insert_backup_config(config.clone())?;
        if let Some(password) = &password {
            if let Err(err) = ui::utils::password_storage::store_password(&config, password).await {
                // Error when storing the password. The repository has already been created, therefore we must continue at this point.
                err.show().await;
            }
        }

        Ok(config.id)
    }

    pub async fn add(&self) -> Result<config::ConfigId> {
        let guard = QuitGuard::default();

        // Show password page
        self.pending_check();

        let repo = self.add_task.repo().unwrap();

        let mut borg = borg::CommandOnlyRepo::new(repo.clone());

        if !self.ask_password.text().is_empty() {
            borg.password = Some(config::Password::new(self.ask_password.text().to_string()));
        }

        let result =
            ui::utils::borg::exec_repo_only(&gettext("Loading Backup Repository"), borg, |borg| {
                borg.peek()
            })
            .await;

        if matches!(
            result,
            Err(ui::error::Combined::Borg(borg::Error::Failed(
                borg::Failure::PassphraseWrong
            )))
        ) {
            self.ask_password();

            return Err(Error::UserCanceled);
        }

        if result.is_err() {
            self.navigation_view.pop_to_page(&*self.location_page);
        }

        let info = result.into_message(gettext("Failed to Configure Repository"))?;

        let encrypted = !self.ask_password.text().is_empty();

        let config = config::Backup::new(repo.clone(), info, encrypted);
        Self::insert_backup_config(config.clone())?;
        main_ui().view_backup_conf(&config.id);
        ui::utils::password_storage::store_password(
            &config,
            &config::Password::new(self.ask_password.text().to_string()),
        )
        .await?;

        self.navigation_view.push(&*self.page_transfer);
        let mut list_command = borg::Command::<borg::task::List>::new(config.clone());
        list_command.task.set_limit_first(100);

        let archives = ui::utils::borg::exec(list_command, &guard)
            .await
            .into_message(gettext("Failed"))?;

        self.transfer_selection(config.id.clone(), archives);

        Ok(config.id)
    }

    fn insert_backup_config(config: config::Backup) -> Result<()> {
        BACKUP_CONFIG.try_update(move |s| {
            s.insert(config.clone())?;
            Ok(())
        })
    }
}
