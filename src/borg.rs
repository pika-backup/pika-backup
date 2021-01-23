pub mod error;
pub mod prelude;
mod utils;
pub use error::Error;
pub mod msg;

use arc_swap::ArcSwap;

use std::io::{BufRead, BufReader};
use std::sync::Arc;

use crate::config::{self, *};
use crate::globals::*;
use crate::prelude::*;
use msg::*;
use utils::*;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Default, Debug, Clone)]
pub struct Status {
    pub run: Run,
    pub last_message: Option<Progress>,
    pub estimated_size: Option<u64>,
}

#[derive(Debug, Clone, Copy)]
pub enum Run {
    Init,
    SizeEstimation,
    Running,
    Reconnecting,
    Stopping,
}

impl Default for Run {
    fn default() -> Self {
        Self::Init
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Stats {
    pub archive: StatsArchive,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StatsArchive {
    duration: f64,
    id: String,
    name: String,
    pub stats: StatsArchiveStats,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StatsArchiveStats {
    pub compressed_size: u64,
    pub deduplicated_size: u64,
    pub nfiles: u64,
    pub original_size: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct List {
    pub archives: Vec<ListArchive>,
    pub encryption: Encryption,
    pub repository: Repository,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListArchive {
    pub id: String,
    pub name: String,
    pub comment: String,
    pub username: String,
    pub hostname: String,
    pub start: chrono::naive::NaiveDateTime,
    pub end: chrono::naive::NaiveDateTime,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Encryption {
    pub mode: String,
    pub keyfile: Option<std::path::PathBuf>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Repository {
    pub id: String,
    pub last_modified: chrono::naive::NaiveDateTime,
    pub location: std::path::PathBuf,
}

#[derive(Clone)]
pub struct Borg {
    config: BackupConfig,
    password: Option<Password>,
}

#[derive(Clone)]
pub struct BorgOnlyRepo {
    repo: BackupRepo,
    password: Option<Password>,
}

pub trait BorgRunConfig {
    fn get_repo(&self) -> BackupRepo;
    fn get_password(&self) -> Option<Password>;
    fn unset_password(&mut self);
    fn set_password(&mut self, password: Password);
    fn is_encrypted(&self) -> bool;
    fn get_config_id(&self) -> Option<String>;
}

impl BorgRunConfig for Borg {
    fn get_repo(&self) -> BackupRepo {
        self.config.repo.clone()
    }
    fn get_password(&self) -> Option<Password> {
        self.password.clone()
    }
    fn set_password(&mut self, password: Password) {
        self.password = Some(password);
    }
    fn unset_password(&mut self) {
        self.password = None;
    }
    fn is_encrypted(&self) -> bool {
        self.config.encrypted
    }
    fn get_config_id(&self) -> Option<String> {
        Some(self.config.id.clone())
    }
}

impl BorgRunConfig for BorgOnlyRepo {
    fn get_repo(&self) -> BackupRepo {
        self.repo.clone()
    }
    fn get_password(&self) -> Option<Password> {
        self.password.clone()
    }
    fn set_password(&mut self, password: Password) {
        self.password = Some(password);
    }
    fn unset_password(&mut self) {
        self.password = None;
    }
    fn is_encrypted(&self) -> bool {
        false
    }
    fn get_config_id(&self) -> Option<String> {
        None
    }
}

/// Features that need a complete backup config
impl Borg {
    pub fn new(config: BackupConfig) -> Self {
        Self {
            config,
            password: None,
        }
    }

    pub fn get_config(&self) -> BackupConfig {
        self.config.clone()
    }

    pub fn umount(&self) -> Result<()> {
        let mount_point = self.get_mount_point();

        let borg = BorgCall::new("umount")
            .add_options(&["--log-json"])
            .add_positional(&mount_point.to_string_lossy())
            .output()?;

        check_stderr(&borg)?;

        std::fs::remove_dir(mount_point)?;
        let _ = std::fs::remove_dir(Self::get_mount_dir());

        Ok(())
    }

    pub fn get_mount_dir() -> std::path::PathBuf {
        HOME_DIR.clone().join(crate::REPO_MOUNT_DIR)
    }

    pub fn get_mount_point(&self) -> std::path::PathBuf {
        let mut dir = Self::get_mount_dir();
        dir.push(&format!("{:.8}", self.config.id));
        dir
    }

    pub fn mount(&self) -> Result<()> {
        std::fs::DirBuilder::new()
            .recursive(true)
            .create(self.get_mount_point())?;

        let borg = BorgCall::new("mount")
            .add_basics(self)?
            .add_positional(&self.get_mount_point().to_string_lossy())
            .output()?;

        check_stderr(&borg)?;

        Ok(())
    }

    pub fn create(&self, communication: Communication) -> Result<Stats> {
        self.create_internal(communication, false)
    }

    fn create_internal(&self, communication: Communication, retry: bool) -> Result<Stats> {
        // Do this early to fail if password is missing
        let mut borg_call = BorgCall::new("create");
        borg_call
            .add_options(&["--progress", "--json"])
            .add_basics(self)?
            .add_archive(self)
            .add_include_exclude(self);

        if retry {
            borg_call.add_options(&[
                "--lock-wait",
                &crate::BORG_LOCK_WAIT_RECONNECT.as_secs().to_string(),
            ]);
        }

        if communication.status.load().estimated_size.is_none() && !retry {
            communication
                .status
                .update(|status| status.run = Run::SizeEstimation);

            let estimated_size = estimate_size(&self.config, &communication);

            if estimated_size.is_some() {
                communication.status.update(move |status| {
                    status.estimated_size = estimated_size;
                });
            } else {
                return Err(Error::UserAborted);
            }
        }

        communication.status.update(move |status| {
            status.run = Run::Running;
        });
        let mut borg = borg_call.spawn()?;

        let mut errors = Vec::new();
        let mut line = String::new();
        let mut reader = BufReader::new(
            borg.stderr
                .take()
                .ok_or_else(|| String::from("Failed to get stderr."))?,
        );
        while reader.read_line(&mut line).unwrap_or(0) > 0 {
            if let Instruction::Abort = **communication.instruction.load() {
                communication.status.update(|status| {
                    status.run = Run::Stopping;
                });
                debug!("Sending SIGTERM to borg::create");
                nix::sys::signal::kill(
                    nix::unistd::Pid::from_raw(borg.id() as i32),
                    nix::sys::signal::Signal::SIGTERM,
                )?;
                borg.wait()?;
                return Err(Error::UserAborted);
            }

            if let Ok(ref msg) = serde_json::from_str::<msg::Progress>(&line) {
                trace!("borg::create: {:?}", msg);
                communication.status.update(move |status| {
                    status.last_message = Some(msg.clone());
                });
            } else {
                let msg = check_line(&line);
                if msg.has_borg_msgid(&MsgId::ConnectionClosed) {
                    communication.status.update(|status| {
                        status.run = Run::Reconnecting;
                    });
                    borg.wait()?;
                    std::thread::sleep(crate::BORG_DELAY_RECONNECT);
                    return self.create_internal(communication, true);
                }
                errors.push(msg);
            }

            line.clear();
        }

        let output = borg.wait_with_output()?;
        let exit_status = output.status;
        debug!("borg::create exited with {:?}", exit_status.code());

        if exit_status.success() {
            let stats: Stats = serde_json::from_slice(&output.stdout)?;
            info!("Stats: {:#?}", stats);
            Ok(stats)
        } else {
            Err(if errors.is_empty() {
                error::ReturnCodeErr::new(exit_status.code()).into()
            } else {
                LogMessageCollection::new(errors).into()
            })
        }
    }
}

impl BorgOnlyRepo {
    pub fn new(repo: BackupRepo) -> Self {
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

    Ok(String::from_utf8_lossy(&borg.stdout).to_string())
}

#[derive(Default, Debug, Clone)]
pub struct Communication {
    pub status: Arc<ArcSwap<Status>>,
    pub instruction: Arc<ArcSwap<Instruction>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Instruction {
    Nothing,
    AbortSizeEstimation,
    Abort,
}

impl Default for Instruction {
    fn default() -> Self {
        Self::Nothing
    }
}

fn pathmatch(entry: &walkdir::DirEntry, pattern: &config::Pattern) -> bool {
    match pattern {
        config::Pattern::PathPrefix(path) => entry.path() == path,
    }
}

pub fn estimate_size(backup: &config::BackupConfig, communication: &Communication) -> Option<u64> {
    let mut exclude = backup.exclude_dirs_internal();

    // Exclude .cache/borg
    if let Some(cache_dir) = glib::get_user_cache_dir() {
        exclude.push(config::Pattern::PathPrefix(
            cache_dir.join(std::path::Path::new("borg")),
        ));
    }

    let is_not_exluded = |e: &walkdir::DirEntry| !exclude.iter().any(|x| pathmatch(e, x));

    let mut size = 0;

    for dir in backup.include_dirs() {
        for entry in walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_entry(is_not_exluded)
            .filter_map(std::result::Result::ok)
        {
            if Instruction::Nothing != **communication.instruction.load() {
                return None;
            }

            if entry.file_type().is_dir() {
                // Empirical value for the space that borg needs
                size += 109;
            } else if let Ok(metadata) = entry.metadata() {
                size += metadata.len();
            }
        }
    }

    debug!("Estimated size {}", &size);
    Some(size)
}
