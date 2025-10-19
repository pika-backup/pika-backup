use std::collections::BTreeSet;

use gio::prelude::*;

use super::{ArchiveParams, SetupAction, SetupCommandLineArgs, SetupRepoLocation};
use crate::ui::prelude::*;
use crate::{borg, config, ui};

pub async fn create_repo_config(
    action: SetupAction,
    location: SetupRepoLocation,
    args: SetupCommandLineArgs,
) -> Result<config::Repository> {
    // A location can either be a borg remote ssh URI or a gio::File
    let mut repo = match location {
        SetupRepoLocation::Remote(url) => {
            // A remote config can only be verified by running borg and checking if it works
            debug!("Creating remote repository config with uri: {}", url);
            config::remote::Repository::from_uri(url).into_config()
        }
        SetupRepoLocation::Local(file) => {
            // A local repo can be either:
            //  * Regular file that is not mounted via gvfs
            //  * GVFS URI
            let uri = file.uri().to_string();

            // If we are creating a new repository we need to use the parent directory for
            // the mount check, because the repo dir does not exist yet
            let mount_file = if action == SetupAction::Init {
                file.parent().unwrap_or_else(|| file.clone())
            } else {
                file.clone()
            };

            // Check if the file is contained in a [`gio::Mount`]
            let mount = mount_file.find_enclosing_mount(Some(&gio::Cancellable::new()));
            debug!("Find mount for '{}': {:?}", mount_file.uri(), mount);

            // Check if we have an actual path already
            let path = if let Some(path) = file.path() {
                path
            } else {
                // We don't. Let's try to mount the URI
                ui::repo::mount_enclosing(&mount_file).await?;

                file.path().ok_or_else(|| {
                    warn!(
                        "Finding enclosing mount failed. URI: '{}', mount result: {:?}",
                        uri, mount
                    );
                    Error::Message(Message::new(
                        gettext("Repository location not found."),
                        gettext(
                            "A mount operation succeeded but the location is still unavailable.",
                        ),
                    ))
                })?
            };

            if let Ok(mount) = mount {
                // We found a mount
                debug!(
                    "Creating local repository config with mount: '{}', path: {:?}, uri: {:?}",
                    mount.name(),
                    path,
                    uri
                );
                config::local::Repository::from_mount(mount, path, uri).into_config()
            } else {
                // We have a path, but we couldn't find a [`gio::Mount`] to go with it.
                // We resort to just store the path.
                //
                // Note: Not storing a mount disables GVFS features, such as detecting drives
                // that have been renamed, or being able to mount the repository location
                // ourselves. This is not the best configuration.
                debug!("Creating local repository config with path: {:?}", path);
                config::local::Repository::from_path(path).into_config()
            }
        }
    };

    // Add command line arguments to repository config if given
    let args_vec = args.into_inner();
    if !args_vec.is_empty() {
        repo.set_settings(Some(config::BackupSettings {
            command_line_args: Some(args_vec),
        }));
    }

    Ok(repo)
}

#[derive(Debug)]
pub enum ConnectRepoError {
    PasswordWrong,
    Error(crate::ui::error::Combined),
}

impl std::fmt::Display for ConnectRepoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectRepoError::PasswordWrong => write!(f, "{}", gettext("Invalid Password")),
            ConnectRepoError::Error(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for ConnectRepoError {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        match self {
            ConnectRepoError::PasswordWrong => None,
            ConnectRepoError::Error(err) => Some(err),
        }
    }
}

/// Validate the password of the repository and try to fetch an archive list.
pub async fn try_peek(
    repo: config::Repository,
    password: Option<config::Password>,
) -> std::result::Result<borg::List, ConnectRepoError> {
    // We connect to the repository to validate the password and retrieve its
    // parameters
    let mut borg = borg::CommandOnlyRepo::new(repo.clone());
    borg.password = password;

    let result =
        ui::utils::borg::exec_repo_only(&gettext("Loading Backup Repository"), borg, |borg| {
            borg.peek()
        })
        .await;

    match result {
        Ok(info) => Ok(info),
        Err(ui::error::Combined::Borg(borg::Error::Failed(borg::Failure::PassphraseWrong))) => {
            // The password was wrong. Let's ask for the password again.
            Err(ConnectRepoError::PasswordWrong)
        }
        Err(err) => {
            // Some other error occurred -> we abort the entire process
            Err(ConnectRepoError::Error(err))
        }
    }
}

pub async fn fetch_archive_list(config: &config::Backup) -> CombinedResult<Vec<borg::ListArchive>> {
    let guard = QuitGuard::default();
    let mut list_command = borg::Command::<borg::task::List>::new(config.clone());
    list_command.task.set_limit_first(100);

    ui::utils::borg::exec(list_command, &guard).await
}

pub async fn transfer_settings(
    config: &mut config::Backup,
    archive_params: &ArchiveParams,
) -> Result<config::ArchivePrefix> {
    config.include.clone_from(&archive_params.parsed.include);
    config.exclude = BTreeSet::from_iter(
        archive_params
            .parsed
            .exclude
            .clone()
            .into_iter()
            .map(|x| x.into_relative()),
    );

    let entry = config::history::RunInfo {
        end: archive_params
            .end
            .and_local_timezone(chrono::Local)
            .unwrap(),
        outcome: borg::Outcome::Completed {
            stats: archive_params.stats.clone(),
        },
        messages: Default::default(),
        include: archive_params.parsed.include.clone(),
        exclude: archive_params.parsed.exclude.clone(),
    };

    // Create fake history entry for duration estimate to be good for first run
    let config_id = &config.id;
    BACKUP_HISTORY
        .try_update(enclose!((config_id) move |histories| {
            histories.insert(config_id.clone(), entry.clone());
            Ok(())
        }))
        .await?;

    let prefix = if let Some(prefix) = &archive_params.prefix {
        prefix.clone()
    } else {
        config.archive_prefix.clone()
    };

    Ok(prefix)
}

pub async fn init_new_backup_repo(
    repo: crate::config::Repository,
    password: &Option<crate::config::Password>,
) -> Result<crate::config::Backup> {
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
    let info =
        ui::utils::borg::exec_repo_only(&gettext("Getting Repository Information"), borg, |borg| {
            borg.peek()
        })
        .await
        .into_message("Failed to Obtain Repository Information")?;

    Ok(config::Backup::new(repo.clone(), info, password.is_some()))
}
