use super::task::Task;
use super::*;
use crate::config;
use crate::config::UserScriptKind;
use crate::prelude::*;
use crate::schedule;
use async_std::prelude::*;
use process::*;
use std::os::unix::fs::DirBuilderExt;
use utils::*;

#[derive(Clone)]
pub struct Command<T: Task> {
    pub config: config::Backup,
    pub communication: Communication<T>,
    pub from_schedule: Option<schedule::DueCause>,
    password: Option<config::Password>,
    pub task: T,
}

#[async_trait]
pub trait CommandRun<T: Task>: Clone + BorgRunConfig {
    async fn run(self) -> Result<T::Return>;
}

impl<T: Task> Command<T> {
    pub fn new(config: config::Backup) -> Self {
        Self {
            config,
            communication: Communication::default(),
            from_schedule: None,
            password: None,
            task: T::default(),
        }
    }

    pub fn set_from_schedule(mut self, from_schedule: Option<schedule::DueCause>) -> Self {
        let is_schedule = from_schedule.is_some();
        self.communication
            .general_info
            .update(move |s| s.is_schedule = is_schedule);

        self.from_schedule = from_schedule;

        self
    }
}

#[async_trait]
impl CommandRun<task::List> for Command<task::List> {
    async fn run(self) -> Result<Vec<ListArchive>> {
        let mut borg = BorgCall::new("list");

        borg.add_options([
            "--json",
            "--consider-checkpoints",
            "--format={hostname}{username}{comment}{end}{command_line}",
        ])
        .add_basics(&self)
        .await?;

        match self.task.limit {
            task::NumArchives::First(n) => {
                borg.add_options([format!("--last={n}")]);
            }
            task::NumArchives::All => (),
        }

        let json: List = borg.output(&self.communication).await?;
        Ok(json.archives)
    }
}

#[async_trait]
impl CommandRun<task::Mount> for Command<task::Mount> {
    async fn run(self) -> Result<()> {
        let dir = mount_point(&self.config.repo_id);
        debug!("Ensuring mount directory exists: {dir:?}");

        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&dir)?;

        let mut borg = BorgCall::new("mount");
        borg.add_basics(&self)
            .await?
            // Also mount incomplete archives (checkpoints)
            .add_options(["--consider-checkpoints"])
            // Make all data readable for the current user
            // <https://gitlab.gnome.org/World/pika-backup/-/issues/132>
            .add_options(["-o", &format!("umask=0000,uid={}", nix::unistd::getuid())])
            .add_positional(&dir);
        borg.output(&self.communication).await?;

        Ok(())
    }
}

#[async_trait]
impl CommandRun<task::PruneInfo> for Command<task::PruneInfo> {
    async fn run(self) -> Result<PruneInfo> {
        let mut borg_call = prune_call(&self).await?;
        borg_call.add_options(["--dry-run", "--list"]);

        borg_call.output(&self.communication).await?;

        let messages = self
            .communication
            .general_info
            .load()
            .all_combined_message_history();

        let list_messages = messages
            .iter()
            .filter_map(|x| {
                if let log_json::LogEntry::ParsedErr(msg) = x {
                    Some(msg)
                } else {
                    None
                }
            })
            .filter(|x| x.name == "borg.output.list");

        let prune = list_messages
            .clone()
            .filter(|x| x.message.starts_with("Would prune"))
            .count();
        let keep = list_messages
            .filter(|x| x.message.starts_with("Keeping"))
            .count();

        Ok(PruneInfo { keep, prune })
    }
}

#[async_trait]
impl CommandRun<task::Prune> for Command<task::Prune> {
    async fn run(self) -> Result<()> {
        let mut borg_call = prune_call(&self).await?;
        borg_call.add_options(["--progress"]);

        borg_call.output(&self.communication).await
    }
}

#[async_trait]
impl CommandRun<task::Compact> for Command<task::Compact> {
    async fn run(self) -> Result<()> {
        let mut borg_call = compact_call(&self).await?;
        borg_call.add_options(["--progress"]);

        borg_call.output(&self.communication).await
    }
}

#[async_trait]
impl CommandRun<task::Check> for Command<task::Check> {
    async fn run(self) -> Result<()> {
        let mut borg_call = check_call(&self).await?;
        borg_call.add_options(["--progress"]);

        if self.task.verify_data() {
            borg_call.add_options(["--verify-data"]);
        }

        if self.task.repair() {
            borg_call.add_options(["--repair"]);
        }

        borg_call.output(&self.communication).await
    }
}

#[async_trait]
impl CommandRun<task::Delete> for Command<task::Delete> {
    async fn run(self) -> Result<()> {
        let archive_name = self.task.archive_name().unwrap_or_default();

        let mut borg_call = delete_call(&self, &archive_name).await?;
        borg_call.add_options(["--progress"]);

        borg_call.output(&self.communication).await
    }
}

#[async_trait]
impl CommandRun<task::Create> for Command<task::Create> {
    async fn run(self) -> Result<Stats> {
        if self.config.include.is_empty() {
            return Err(Error::EmptyInclude);
        }

        let mut borg_call = BorgCall::new("create");
        borg_call
            .add_options(["--progress", "--json"])
            // Good and fast compression
            // <https://gitlab.gnome.org/World/pika-backup/-/issues/51>
            .add_options(&["--compression=zstd"])
            .add_basics(&self)
            .await?
            .add_archive(&self)
            .add_include_exclude(&self);

        let process = borg_call.spawn_background(&self.communication)?;

        let mut last_skipped = 0.;
        let mut last_copied = 0.;
        let mut last_time = std::time::Instant::now();

        self.communication.specific_info.update(move |status| {
            status.started = Some(chrono::Local::now());
        });

        let mut log = self.communication.new_receiver();

        while let Some(msg) = log.next().await {
            trace!("borg::create: {:?}", msg);

            if let Update::Msg(log_json::Output::Progress(log_json::Progress::Archive(
                ref progress,
            ))) = msg
            {
                let skipped = progress.original_size as f64 - progress.deduplicated_size as f64;
                let copied = progress.deduplicated_size as f64;
                let interval = last_time.elapsed().as_secs_f64();
                last_time = std::time::Instant::now();

                self.communication.specific_info.update(move |status| {
                    status.total = progress.original_size as f64;
                    status.copied = progress.deduplicated_size as f64;

                    status.data_rate_history.insert(DataRate {
                        interval,
                        skipped: skipped - last_skipped,
                        copied: copied - last_copied,
                    });
                });

                last_skipped = skipped;
                last_copied = copied;
            }
        }

        process.result.await
    }
}

#[async_trait]
impl CommandRun<task::KeyChangePassphrase> for Command<task::KeyChangePassphrase> {
    async fn run(self) -> Result<()> {
        let Some(new_password) = self.task.new_password() else {
            return Err(Error::from("The new password wasn't set".to_string()));
        };

        let mut borg_call = BorgCall::new("key");
        borg_call
            .add_sub_command("change-passphrase")
            .add_basics(&self)
            .await?
            .add_envs([(
                "BORG_NEW_PASSPHRASE",
                std::str::from_utf8(new_password.to_owned().as_bytes())
                    .map_err(|_| Error::from("The new password is not valid UTF-8".to_string()))?,
            )]);

        // TODO: Use spawn_managed. The lack of properly tagged output unfortunately means that a
        // non-zero return code wouldn't be considered an error by that function.
        info!("Running borg: {:#?}", borg_call);
        let output: RawOutput = borg_call.output_generic().await?;

        let stdout = String::from_utf8_lossy(&output.output);
        debug!("stdout: {}", stdout);

        // TODO: Why is this on stdout?
        if stdout.contains("repository is not encrypted") {
            return Err(Failure::Other(gettext("This backup repository is not encrypted at all, not even with a key stored in plain text. Passwords can only be added to repositories that use encryption.")).into());
        }

        Ok(())
    }
}

#[async_std::test]
async fn create_non_existent_location() {
    let config = config::Backup::test_new_mock();

    let result = Command::<task::Create>::new(config).run().await;
    matches::assert_matches!(
        result,
        Err(error::Error::Failed(error::Failure::RepositoryDoesNotExist))
    );
}

#[async_trait]
impl CommandRun<task::UserScript> for Command<task::UserScript> {
    async fn run(self) -> Result<()> {
        let Some(kind) = self.task.kind() else {
            return Err(Error::from(
                "The UserScript task kind wasn't set".to_string(),
            ));
        };

        let Some(script) = self.config.user_scripts.get(&kind) else {
            // We don't have a script action configured in the config, so we don't do anything
            return Ok(());
        };

        let env = match kind {
            UserScriptKind::PreBackup => {
                super::scripts::script_env_pre(&self.config, self.from_schedule.is_some())
            }
            UserScriptKind::PostBackup => {
                let Some(run_info) = self.task.run_info() else {
                    return Err(Error::from(
                        "The UserScript task RunInfo wasn't set".to_string(),
                    ));
                };

                super::scripts::script_env_post(
                    &self.config,
                    self.from_schedule.is_some(),
                    run_info,
                )
            }
        };

        super::scripts::run_script(script, env, kind, self.communication).await
    }
}

#[derive(Clone)]
pub struct CommandOnlyRepo {
    repo: config::Repository,
    pub password: Option<config::Password>,
}

pub trait BorgRunConfig: Clone + Send + 'static {
    fn repo(&self) -> config::Repository;
    fn password(&self) -> Option<config::Password>;
    fn unset_password(&mut self);
    fn set_password(&mut self, password: config::Password);
    fn is_encrypted(&self) -> bool;
    fn config_id(&self) -> Option<ConfigId>;
    fn try_config(&self) -> Option<config::Backup>;
}

impl<T: Task> BorgRunConfig for Command<T> {
    fn repo(&self) -> config::Repository {
        self.config.repo.clone()
    }

    fn password(&self) -> Option<config::Password> {
        self.password.clone()
    }

    fn set_password(&mut self, password: config::Password) {
        self.password = Some(password);
    }

    fn unset_password(&mut self) {
        self.password = None;
    }

    fn is_encrypted(&self) -> bool {
        self.config.encrypted
    }

    fn config_id(&self) -> Option<ConfigId> {
        Some(self.config.id.clone())
    }

    fn try_config(&self) -> Option<config::Backup> {
        Some(self.config.clone())
    }
}

impl BorgRunConfig for CommandOnlyRepo {
    fn repo(&self) -> config::Repository {
        self.repo.clone()
    }

    fn password(&self) -> Option<config::Password> {
        self.password.clone()
    }

    fn set_password(&mut self, password: config::Password) {
        self.password = Some(password);
    }

    fn unset_password(&mut self) {
        self.password = None;
    }

    fn is_encrypted(&self) -> bool {
        false
    }

    fn config_id(&self) -> Option<ConfigId> {
        None
    }

    fn try_config(&self) -> Option<config::Backup> {
        None
    }
}

#[derive(Clone, Debug)]
pub struct PruneInfo {
    pub keep: usize,
    pub prune: usize,
}

pub async fn is_mounted(repo_id: &RepoId) -> bool {
    let mount_point = mount_point(repo_id);

    // Check if the directory is still a mountpoint (otherwise it was unmounted via other means)
    async_std::task::spawn_blocking(move || {
        gio::UnixMountEntry::for_mount_path(mount_point).0.is_some()
    })
    .await
}

pub async fn umount(repo_id: &RepoId) -> Result<()> {
    let mount_point = mount_point(repo_id);

    if is_mounted(repo_id).await {
        BorgCall::new("umount")
            .add_options(["--log-json"])
            .add_positional(&mount_point)
            .output_generic::<()>()
            .await?;
    }

    if let Err(err) = async_std::fs::remove_dir(mount_point).await {
        match err.kind() {
            std::io::ErrorKind::NotFound => {
                // If the dir didn't exist in the first place we shouldn't throw an error
                warn!("Error when removing mount dir: {:?}", err);
            }
            _ => return Err(err.into()),
        }
    }

    // Other mounts could exist that still use the dir. We just clean it up if possible.
    if let Err(err) = async_std::fs::remove_dir(mount_base_dir()).await {
        debug!("Error when removing mount base dir: {:?}", err);
    }

    Ok(())
}

pub fn mount_point(repo_id: &RepoId) -> std::path::PathBuf {
    let mut dir = mount_base_dir();
    dir.push(&format!("{:.8}", repo_id.as_str()));
    dir
}

async fn prune_call<T: Task>(command: &Command<T>) -> Result<BorgCall> {
    if command.config.prune.keep.hourly < 1
        || command.config.prune.keep.daily < 1
        || command.config.prune.keep.weekly < 1
    {
        return Err(Error::ImplausiblePrune);
    }

    let mut borg_call = BorgCall::new("prune");

    borg_call.add_basics(command).await?.add_options([
        &format!("--glob-archives={}*", command.config.archive_prefix),
        "--keep-within=1H",
        &format!("--keep-hourly={}", command.config.prune.keep.hourly),
        &format!("--keep-daily={}", command.config.prune.keep.daily),
        &format!("--keep-weekly={}", command.config.prune.keep.weekly),
        &format!("--keep-monthly={}", command.config.prune.keep.monthly),
        &format!("--keep-yearly={}", command.config.prune.keep.yearly),
    ]);

    Ok(borg_call)
}

async fn delete_call<T: Task>(command: &Command<T>, archive_name: &str) -> Result<BorgCall> {
    let mut borg_call = BorgCall::new("delete");

    borg_call
        .add_basics(command)
        .await?
        .add_positional(archive_name);
    Ok(borg_call)
}

async fn compact_call<T: Task>(command: &Command<T>) -> Result<BorgCall> {
    let mut borg_call = BorgCall::new("compact");

    borg_call.add_basics(command).await?;

    Ok(borg_call)
}

async fn check_call<T: Task>(command: &Command<T>) -> Result<BorgCall> {
    let mut borg_call = BorgCall::new("check");

    borg_call.add_basics(command).await?;
    Ok(borg_call)
}

/// Features that are available without complete backup config
impl CommandOnlyRepo {
    pub const fn new(repo: config::Repository) -> Self {
        Self {
            repo,
            password: None,
        }
    }

    pub async fn break_lock(self) -> Result<()> {
        BorgCall::new("break-lock")
            .add_basics_without_password(&self)
            .output_generic()
            .await
    }

    pub async fn peek(self) -> Result<List> {
        BorgCall::new("list")
            .add_options([
                "--json",
                "--last=1",
                "--format={hostname}{username}{comment}{end}{command_line}",
            ])
            .add_envs(vec![
                ("BORG_UNKNOWN_UNENCRYPTED_REPO_ACCESS_IS_OK", "yes"),
                ("BORG_RELOCATED_REPO_ACCESS_IS_OK", "yes"),
            ])
            .add_basics(&self)
            .await?
            .output_generic()
            .await
    }

    pub async fn init(self) -> Result<List> {
        BorgCall::new("init")
            .add_options([format!("--encryption=repokey{}", fasted_hash_algorithm()).as_str()])
            .add_basics(&self)
            .await?
            .output_generic::<()>()
            .await?;

        self.peek().await
    }
}

pub async fn version() -> Result<String> {
    let borg: RawOutput = BorgCall::new_raw()
        .add_options(["--log-json", "--version"])
        .output_generic()
        .await?;

    Ok(String::from_utf8_lossy(&borg.output).trim().to_string())
}
