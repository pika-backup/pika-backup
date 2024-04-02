use adw::prelude::*;
use adw::subclass::prelude::*;

use super::imp;
use super::remote_location::RemoteLocation;
use crate::borg;
use crate::borg::prelude::*;
use crate::config;
use crate::config::*;
use crate::ui;
use crate::ui::prelude::*;

impl imp::SetupDialog {
    pub async fn add_button_clicked(&self) -> Result<()> {
        let remote_location = RemoteLocation::from_user_input(self.location_url.text().to_string())
            .err_to_msg(gettext("Invalid Remote Location"))?;

        debug!("Add existing URI '{:?}'", remote_location.url());

        let repo = if remote_location.is_borg_host() {
            config::remote::Repository::from_uri(remote_location.url()).into_config()
        } else {
            Self::mount_fuse_and_config(&remote_location.as_gio_file(), false)
                .await?
                .into_config()
        };

        self.add_first_try(repo).await
    }

    pub async fn on_init_button_clicked(&self) -> Result<()> {
        let encryption_result = self.encryption_preferences_group.validated_password();

        if let Err(err) = encryption_result {
            self.navigation_view
                .pop_to_page(&*self.page_setup_encryption);
            return Err(err);
        }

        let result = self.init_repo().await;

        if result.is_ok() {
            self.obj().close();
        } else {
            self.navigation_view.pop_to_page(&*self.page_detail);
        }

        result
    }

    async fn get_repo(&self) -> Result<Repository> {
        if self.location_group_local.is_visible() {
            if let Some(path) = self
                .init_path
                .file()
                .map(|x| x.child(self.init_dir.text().as_str()))
                .and_then(|x| x.path())
            {
                if let Some(mount) = self.init_path.file().and_then(|file| {
                    file.find_enclosing_mount(Some(&gio::Cancellable::new()))
                        .ok()
                }) {
                    let uri = gio::File::for_path(&path).uri().to_string();
                    Ok(local::Repository::from_mount(mount, path, uri).into_config())
                } else {
                    Ok(local::Repository::from_path(path).into_config())
                }
            } else {
                Err(Message::new(
                    gettext("Location is not a valid backup repository."),
                    gettext("A repository location has to be given."),
                )
                .into())
            }
        } else {
            let remote_location =
                RemoteLocation::from_user_input(self.location_url.text().to_string())
                    .err_to_msg(gettext("Invalid Remote Location"))?;

            if remote_location.is_borg_host() {
                // Do not mount since borg will create a direct SSH connection
                Ok(config::remote::Repository::from_uri(remote_location.url()).into_config())
            } else {
                // Mount if necessary
                Self::mount_fuse_and_config(&remote_location.as_gio_file(), true)
                    .await
                    .map(|x| x.into_config())
            }
        }
    }

    pub async fn validate_detail_page(&self) -> Result<()> {
        self.get_repo().await?;
        self.navigation_view.push(&*self.page_setup_encryption);
        Ok(())
    }

    async fn init_repo(&self) -> Result<()> {
        let encrypted = self.encryption_preferences_group.encrypted();
        let password = self.encryption_preferences_group.validated_password()?;

        let mut repo = self.get_repo().await?;

        let args = self.command_line_args()?;
        repo.set_settings(Some(BackupSettings {
            command_line_args: Some(args),
        }));

        self.navigation_view.push(&*self.page_creating);

        let mut borg = borg::CommandOnlyRepo::new(repo.clone());
        if encrypted {
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
        if encrypted {
            if let Err(err) = ui::utils::password_storage::store_password(&config, &password).await
            {
                // Error when storing the password. The repository has already been created, therefore we must continue at this point.
                err.show().await;
            }
        }
        main_ui().view_backup_conf(&config.id);

        Ok(())
    }

    pub async fn add_first_try(&self, mut repo: config::Repository) -> Result<()> {
        repo.set_settings(Some(BackupSettings {
            command_line_args: Some(self.command_line_args()?),
        }));

        self.add_task.set_repo(Some(repo.clone()));

        self.add().await
    }

    pub async fn add(&self) -> Result<()> {
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
            self.navigation_view.pop_to_page(&*self.page_detail);
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

        Ok(())
    }

    fn insert_backup_config(config: config::Backup) -> Result<()> {
        BACKUP_CONFIG.try_update(move |s| {
            s.insert(config.clone())?;
            Ok(())
        })
    }

    pub fn execute<F: std::future::Future<Output = Result<()>> + 'static, W: IsA<gtk::Window>>(
        f: F,
        window: W,
    ) {
        Handler::new().error_transient_for(window).spawn(f);
    }

    fn command_line_args(&self) -> Result<Vec<String>> {
        let text = self.command_line_args_entry.text();
        ui::utils::borg::parse_borg_command_line_args(&text)
    }

    async fn mount_fuse_and_config(
        uri: &gio::File,
        mount_parent: bool,
    ) -> Result<local::Repository> {
        let enclosing_mount = uri.find_enclosing_mount(Some(&gio::Cancellable::new()));
        debug!("Tried to find enclosing mount: {enclosing_mount:?}");

        if let (Ok(mount), Some(path)) = (enclosing_mount, uri.path()) {
            Ok(local::Repository::from_mount(
                mount,
                path,
                uri.uri().to_string(),
            ))
        } else {
            let mount_uri = if mount_parent {
                uri.parent().as_ref().unwrap_or(uri).uri()
            } else {
                uri.uri()
            };

            ui::repo::mount_enclosing(&gio::File::for_uri(&mount_uri)).await?;

            let enclosing_mount = uri.find_enclosing_mount(Some(&gio::Cancellable::new()));
            let path = uri.path();

            if let (Ok(mount), Some(path)) = (enclosing_mount.clone(), path.clone()) {
                Ok(local::Repository::from_mount(
                    mount,
                    path.clone(),
                    uri.uri().to_string(),
                ))
            } else {
                warn!(
                    "Finding enclosing mount failed. Path: '{path:?}'. Mount: {enclosing_mount:?}"
                );
                Err(Error::Message(Message::new(
                    gettext("Repository location not found."),
                    gettext("A mount operation succeeded but the location is still unavailable."),
                )))
            }
        }
    }
}
