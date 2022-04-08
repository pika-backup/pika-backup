use super::task::Task;
use super::*;
use crate::config;
use crate::prelude::*;
use crate::schedule;
use async_std::prelude::*;
use process::*;
use utils::*;

#[derive(Clone)]
pub struct Command<T: Task> {
    pub config: config::Backup,
    pub communication: Communication<T>,
    pub from_schedule: Option<schedule::DueCause>,
    password: Option<config::Password>,
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
        }
    }

    pub fn set_from_schedule(mut self, from_schedule: Option<schedule::DueCause>) -> Self {
        self.from_schedule = from_schedule;

        self
    }
}

#[async_trait]
impl CommandRun<task::List> for Command<task::List> {
    async fn run(self) -> Result<Vec<ListArchive>> {
        let borg = BorgCall::new("list")
            .add_options(&[
                "--json",
                &format!("--last={}", 100),
                "--consider-checkpoints",
                "--format={hostname}{username}{comment}{end}{command_line}",
            ])
            .add_basics(&self)?
            .output()?;

        check_stderr(&borg)?;

        let json: List = serde_json::from_slice(&borg.stdout)?;

        Ok(json.archives)
    }
}

#[async_trait]
impl CommandRun<task::Mount> for Command<task::Mount> {
    async fn run(self) -> Result<()> {
        std::fs::DirBuilder::new()
            .recursive(true)
            .create(mount_point(&self.config.repo_id))?;

        let borg = BorgCall::new("mount")
            .add_basics(&self)?
            // Make all data readable for the current user
            // <https://gitlab.gnome.org/World/pika-backup/-/issues/132>
            .add_options(&["-o", &format!("umask=0277,uid={}", nix::unistd::getuid())])
            .add_positional(&mount_point(&self.config.repo_id).to_string_lossy())
            .output()?;

        check_stderr(&borg)?;

        Ok(())
    }
}

#[async_trait]
impl CommandRun<task::PruneInfo> for Command<task::PruneInfo> {
    async fn run(self) -> Result<PruneInfo> {
        let mut borg_call = prune_call(&self)?;
        borg_call.add_options(&["--dry-run", "--list"]);

        let messages = check_stderr(&borg_call.output()?)?;

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
        let mut borg_call = prune_call(&self)?;
        borg_call.add_options(&["--progress"]);

        let process = borg_call.spawn_async_managed(self.communication.clone())?;

        process.result.await
    }
}

#[async_trait]
impl CommandRun<task::Create> for Command<task::Create> {
    async fn run(self) -> Result<Stats> {
        if self.config.include.is_empty() {
            return Err(Error::EmptyIncldue);
        }

        let mut borg_call = BorgCall::new("create");
        borg_call
            .add_options(&["--progress", "--json"])
            // Good and fast compression
            // <https://gitlab.gnome.org/World/pika-backup/-/issues/51>
            .add_options(&["--compression=zstd"])
            .add_basics(&self)?
            .add_archive(&self)
            .add_include_exclude(&self);

        let process = borg_call.spawn_async_managed(self.communication.clone())?;

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

#[async_std::test]
async fn create_non_existent_location() {
    let config = config::Backup::test_new_mock();

    let result = Command::<task::Create>::new(config).run().await;
    matches::assert_matches!(
        result,
        Err(error::Error::Failed(error::Failure::RepositoryDoesNotExist))
    );
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

pub fn umount(repo_id: &RepoId) -> Result<()> {
    let mount_point = mount_point(repo_id);

    let borg = BorgCall::new("umount")
        .add_options(&["--log-json"])
        .add_positional(&mount_point.to_string_lossy())
        .output()?;

    check_stderr(&borg)?;

    std::fs::remove_dir(mount_point)?;
    let _ = std::fs::remove_dir(mount_dir());

    Ok(())
}

pub fn mount_point(repo_id: &RepoId) -> std::path::PathBuf {
    let mut dir = mount_dir();
    dir.push(&format!("{:.8}", repo_id.as_str()));
    dir
}

pub fn mount_dir() -> std::path::PathBuf {
    glib::home_dir().join(crate::REPO_MOUNT_DIR)
}

fn prune_call<T: Task>(command: &Command<T>) -> Result<BorgCall> {
    if command.config.prune.keep.hourly < 1
        || command.config.prune.keep.daily < 1
        || command.config.prune.keep.weekly < 1
    {
        return Err(Error::ImplausiblePrune);
    }

    let mut borg_call = BorgCall::new("prune");

    borg_call.add_basics(command)?.add_options(&[
        &format!("--prefix={}", command.config.archive_prefix),
        "--keep-within=1H",
        &format!("--keep-hourly={}", command.config.prune.keep.hourly),
        &format!("--keep-daily={}", command.config.prune.keep.daily),
        &format!("--keep-weekly={}", command.config.prune.keep.weekly),
        &format!("--keep-monthly={}", command.config.prune.keep.monthly),
        &format!("--keep-yearly={}", command.config.prune.keep.yearly),
    ]);

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

    pub fn break_lock(&self) -> Result<()> {
        let borg = BorgCall::new("break-lock")
            .add_basics_without_password(self)
            .output()?;
        check_stderr(&borg)?;
        Ok(())
    }

    pub async fn peek(self) -> Result<List> {
        let borg = BorgCall::new("list")
            .add_options(&[
                "--json",
                "--last=1",
                "--format={hostname}{username}{comment}{end}{command_line}",
            ])
            .add_envs(vec![
                ("BORG_UNKNOWN_UNENCRYPTED_REPO_ACCESS_IS_OK", "yes"),
                ("BORG_RELOCATED_REPO_ACCESS_IS_OK", "yes"),
            ])
            .add_basics(&self)?
            .output()?;

        check_stderr(&borg)?;

        let json: List = serde_json::from_slice(&borg.stdout)?;

        Ok(json)
    }

    pub async fn init(self) -> Result<List> {
        let borg = BorgCall::new("init")
            .add_options(&["--encryption=repokey"])
            .add_basics(&self)?
            .output()?;

        check_stderr(&borg)?;

        self.peek().await
    }
}

pub fn version() -> Result<String> {
    let borg = BorgCall::new_raw()
        .add_options(&["--log-json", "--version"])
        .output()?;

    check_stderr(&borg)?;

    Ok(String::from_utf8_lossy(&borg.stdout).trim().to_string())
}
