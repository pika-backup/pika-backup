use futures::prelude::*;

use futures::channel::mpsc::UnboundedSender;

use super::*;
use crate::config;
use crate::prelude::*;
use process::*;
use utils::*;

#[derive(Clone)]
pub struct Borg {
    pub config: config::Backup,
    password: Option<config::Password>,
}

#[derive(Clone)]
pub struct BorgOnlyRepo {
    repo: config::Repository,
    pub password: Option<config::Password>,
}

pub trait BorgRunConfig {
    fn repo(&self) -> config::Repository;
    fn password(&self) -> Option<config::Password>;
    fn unset_password(&mut self);
    fn set_password(&mut self, password: config::Password);
    fn is_encrypted(&self) -> bool;
    fn config_id(&self) -> Option<ConfigId>;
    fn try_config(&self) -> Option<config::Backup>;
}

impl BorgRunConfig for Borg {
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

impl BorgRunConfig for BorgOnlyRepo {
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

/// Features that need a complete backup config
impl Borg {
    pub fn new(config: config::Backup) -> Self {
        Self {
            config,
            password: None,
        }
    }

    pub fn umount(repo_id: &RepoId) -> Result<()> {
        let mount_point = Self::mount_point(repo_id);

        let borg = BorgCall::new("umount")
            .add_options(&["--log-json"])
            .add_positional(&mount_point.to_string_lossy())
            .output()?;

        check_stderr(&borg)?;

        std::fs::remove_dir(mount_point)?;
        let _ = std::fs::remove_dir(Self::mount_dir());

        Ok(())
    }

    pub fn mount_dir() -> std::path::PathBuf {
        glib::home_dir().join(crate::REPO_MOUNT_DIR)
    }

    pub fn mount_point(repo_id: &RepoId) -> std::path::PathBuf {
        let mut dir = Self::mount_dir();
        dir.push(&format!("{:.8}", repo_id.as_str()));
        dir
    }

    pub fn mount(&self) -> Result<()> {
        std::fs::DirBuilder::new()
            .recursive(true)
            .create(Self::mount_point(&self.config.repo_id))?;

        let borg = BorgCall::new("mount")
            .add_basics(self)?
            // Make all data readable for the current user
            // <https://gitlab.gnome.org/World/pika-backup/-/issues/132>
            .add_options(&["-o", &format!("umask=0277,uid={}", nix::unistd::getuid())])
            .add_positional(&Self::mount_point(&self.config.repo_id).to_string_lossy())
            .output()?;

        check_stderr(&borg)?;

        Ok(())
    }

    fn prune_call(&self) -> Result<BorgCall> {
        if self.config.prune.keep.hourly < 1
            || self.config.prune.keep.daily < 1
            || self.config.prune.keep.weekly < 1
        {
            return Err(Error::ImplausiblePrune);
        }

        let mut borg_call = BorgCall::new("prune");

        borg_call.add_basics(self)?.add_options(&[
            "--list",
            &format!("--prefix={}", self.config.archive_prefix),
            "--keep-within=1H",
            &format!("--keep-hourly={}", self.config.prune.keep.hourly),
            &format!("--keep-daily={}", self.config.prune.keep.daily),
            &format!("--keep-weekly={}", self.config.prune.keep.weekly),
            &format!("--keep-monthly={}", self.config.prune.keep.monthly),
            &format!("--keep-yearly={}", self.config.prune.keep.yearly),
        ]);

        Ok(borg_call)
    }

    pub fn prune_info(&self) -> Result<PruneInfo> {
        self.prune_info_()
    }

    fn prune_info_(&self) -> Result<PruneInfo> {
        let mut borg_call = self.prune_call()?;
        borg_call.add_options(&["--dry-run"]);

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
            .filter(|x| x.message.starts_with("Pruning") || x.message.starts_with("Would prune"))
            .count();
        let keep = list_messages
            .filter(|x| x.message.starts_with("Keeping"))
            .count();

        Ok(PruneInfo { keep, prune })
    }

    pub fn prune(
        &self,
        communication: Communication,
        sender: UnboundedSender<(u32, u32)>,
    ) -> Result<()> {
        futures::executor::block_on(self.prune_(communication, sender))
    }

    async fn prune_(
        &self,
        communication: Communication,
        mut sender: UnboundedSender<(u32, u32)>,
    ) -> Result<()> {
        if self.config.prune.keep.hourly < 1
            || self.config.prune.keep.daily < 1
            || self.config.prune.keep.weekly < 1
        {
            return Err(Error::ImplausiblePrune);
        }

        let borg_call = self.prune_call()?;

        let mut process = borg_call.spawn_async_managed(communication)?;

        while let Some(msg) = process.log.next().await {
            if let log_json::Output::LogEntry(log_json::LogEntry::ParsedErr(msg)) = msg {
                if msg.name == "borg.output.list" {
                    if let Some((_, current, total)) =
                        regex_captures!("Pruning archive: .*\\((.*)/(.*)\\)", &msg.message)
                    {
                        debug!("{current} of {total}");
                        if let Ok(status) = current
                            .parse()
                            .and_then(|current| total.parse().map(|total| (current, total)))
                        {
                            let _res = sender.send(status).await;
                        }
                    }
                }
            }
        }

        process.result.await?
    }

    pub fn create(&self, communication: Communication) -> Result<Stats> {
        futures::executor::block_on(self.create_(communication))
    }

    async fn create_(&self, communication: Communication) -> Result<Stats> {
        communication
            .status
            .update(move |status| status.message_history.push(Default::default()));

        let mut borg_call = BorgCall::new("create");
        borg_call
            .add_options(&["--progress", "--json"])
            // Good and fast compression
            // <https://gitlab.gnome.org/World/pika-backup/-/issues/51>
            .add_options(&["--compression=zstd"])
            .add_basics(self)?
            .add_archive(self)
            .add_include_exclude(self);

        let mut process = borg_call.spawn_async_managed(communication.clone())?;

        let mut last_skipped = 0.;
        let mut last_copied = 0.;
        let mut last_time = std::time::Instant::now();

        while let Some(msg) = process.log.next().await {
            trace!("borg::create: {:?}", msg);

            if let log_json::Output::Progress(
                ref archive @ log_json::Progress::Archive(ref progress),
            ) = msg
            {
                // TODO: legacy? could be solved via channel
                communication.status.update(move |status| {
                    status.last_message = Some(archive.clone());
                });

                let skipped = progress.original_size as f64 - progress.deduplicated_size as f64;
                let copied = progress.deduplicated_size as f64;
                let interval = last_time.elapsed().as_secs_f64();
                last_time = std::time::Instant::now();

                communication.status.update(move |status| {
                    status.run = Run::Running;
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

        process.result.await?
    }
}

impl BorgOnlyRepo {
    pub fn new(repo: config::Repository) -> Self {
        Self {
            repo,
            password: None,
        }
    }
}

impl BorgBasics for Borg {}
impl BorgBasics for BorgOnlyRepo {}

/// Features that are available without complete backup config
pub trait BorgBasics: BorgRunConfig + Sized + Clone + Send {
    fn break_lock(&self) -> Result<()> {
        let borg = BorgCall::new("break-lock")
            .add_basics_without_password(self)
            .output()?;
        check_stderr(&borg)?;
        Ok(())
    }

    fn peek(&self) -> Result<List> {
        let borg = BorgCall::new("list")
            .add_options(&[
                "--json",
                "--last=1",
                "--format={hostname}{username}{comment}{end}",
            ])
            .add_envs(vec![
                ("BORG_UNKNOWN_UNENCRYPTED_REPO_ACCESS_IS_OK", "yes"),
                ("BORG_RELOCATED_REPO_ACCESS_IS_OK", "yes"),
            ])
            .add_basics(self)?
            .output()?;

        check_stderr(&borg)?;

        let json: List = serde_json::from_slice(&borg.stdout)?;

        Ok(json)
    }

    fn list(&self, last: u64) -> Result<Vec<ListArchive>> {
        let borg = BorgCall::new("list")
            .add_options(&[
                "--json",
                &format!("--last={}", last),
                "--format={hostname}{username}{comment}{end}",
            ])
            .add_basics(self)?
            .output()?;

        check_stderr(&borg)?;

        let json: List = serde_json::from_slice(&borg.stdout)?;

        Ok(json.archives)
    }

    fn info(&self, last: u64) -> Result<Vec<InfoArchive>> {
        let borg = BorgCall::new("info")
            .add_options(&["--json", &format!("--last={}", last)])
            .add_basics(self)?
            .output()?;

        check_stderr(&borg)?;

        let json: Info = serde_json::from_slice(&borg.stdout)?;

        Ok(json.archives)
    }

    fn init(&self) -> Result<List> {
        let borg = BorgCall::new("init")
            .add_options(&["--encryption=repokey"])
            .add_basics(self)?
            .output()?;

        check_stderr(&borg)?;

        self.peek()
    }
}

pub fn version() -> Result<String> {
    let borg = BorgCall::new_raw()
        .add_options(&["--log-json", "--version"])
        .output()?;

    check_stderr(&borg)?;

    Ok(String::from_utf8_lossy(&borg.stdout).trim().to_string())
}
