use std::io::{BufRead, BufReader};

use super::*;
use crate::config;
use crate::prelude::*;
use msg::*;
use utils::*;

#[derive(Clone)]
pub struct Borg {
    pub config: config::Backup,
    password: Option<config::Password>,
}

#[derive(Clone)]
pub struct BorgOnlyRepo {
    repo: config::Repository,
    password: Option<config::Password>,
}

pub trait BorgRunConfig {
    fn get_repo(&self) -> config::Repository;
    fn get_password(&self) -> Option<config::Password>;
    fn unset_password(&mut self);
    fn set_password(&mut self, password: config::Password);
    fn is_encrypted(&self) -> bool;
    fn get_config_id(&self) -> Option<ConfigId>;
}

impl BorgRunConfig for Borg {
    fn get_repo(&self) -> config::Repository {
        self.config.repo.clone()
    }
    fn get_password(&self) -> Option<config::Password> {
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
    fn get_config_id(&self) -> Option<ConfigId> {
        Some(self.config.id.clone())
    }
}

impl BorgRunConfig for BorgOnlyRepo {
    fn get_repo(&self) -> config::Repository {
        self.repo.clone()
    }
    fn get_password(&self) -> Option<config::Password> {
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
    fn get_config_id(&self) -> Option<ConfigId> {
        None
    }
}

/// Features that need a complete backup config
impl Borg {
    pub fn new(config: config::Backup) -> Self {
        Self {
            config,
            password: None,
        }
    }

    pub fn get_config(&self) -> config::Backup {
        self.config.clone()
    }

    pub fn umount(repo_id: &RepoId) -> Result<()> {
        let mount_point = Self::get_mount_point(repo_id);

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

    pub fn get_mount_point(repo_id: &RepoId) -> std::path::PathBuf {
        let mut dir = Self::get_mount_dir();
        dir.push(&format!("{:.8}", repo_id.as_str()));
        dir
    }

    pub fn mount(&self) -> Result<()> {
        std::fs::DirBuilder::new()
            .recursive(true)
            .create(Self::get_mount_point(&self.config.repo_id))?;

        let borg = BorgCall::new("mount")
            .add_basics(self)?
            .add_positional(&Self::get_mount_point(&self.config.repo_id).to_string_lossy())
            .output()?;

        check_stderr(&borg)?;

        Ok(())
    }

    pub fn create(&self, communication: Communication) -> Result<Stats> {
        self.create_internal(communication, 0)
    }

    fn create_internal(&self, communication: Communication, retries: u16) -> Result<Stats> {
        communication
            .status
            .update(move |status| status.message_history.push(Default::default()));

        // Do this early to fail if password is missing
        let mut borg_call = BorgCall::new("create");
        borg_call
            .add_options(&["--progress", "--json"])
            .add_basics(self)?
            .add_archive(self)
            .add_include_exclude(self);

        if retries > 0 {
            borg_call.add_options(&[
                "--lock-wait",
                &crate::BORG_LOCK_WAIT_RECONNECT.as_secs().to_string(),
            ]);
        }

        communication.status.update(move |status| {
            status.run = Run::Running;
        });
        let mut borg = borg_call.spawn()?;

        let mut last_skipped = 0.;
        let mut last_copied = 0.;
        let mut last_time = std::time::Instant::now();

        communication.status.update(move |status| {
            status.started = Some(chrono::Local::now());
        });

        let mut line = String::new();
        let mut reader = BufReader::new(
            borg.stderr
                .take()
                .ok_or_else(|| String::from("Failed to get stderr."))?,
        );

        while reader.read_line(&mut line)? > 0 {
            if matches!(**communication.instruction.load(), Instruction::Abort) {
                communication.status.update(|status| {
                    status.run = Run::Stopping;
                });
                debug!("Sending SIGTERM to borg::create");
                nix::sys::signal::kill(
                    nix::unistd::Pid::from_raw(borg.id() as i32),
                    nix::sys::signal::Signal::SIGTERM,
                )?;
                borg.wait()?;
                return Err(Error::Aborted(error::Abort::User));
            }

            if let Ok(ref msg) = serde_json::from_str::<msg::Progress>(&line) {
                trace!("borg::create: {:?}", msg);

                if let msg::Progress::Archive(progress) = msg {
                    let skipped = progress.original_size as f64 - progress.deduplicated_size as f64;
                    let copied = progress.deduplicated_size as f64;
                    let interval = last_time.elapsed().as_secs_f64();
                    last_time = std::time::Instant::now();

                    communication.status.update(move |status| {
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

                communication.status.update(move |status| {
                    status.last_message = Some(msg.clone());
                });
            } else {
                let msg = check_line(&line);
                if msg.is_connection_error() && retries <= crate::BORG_MAX_RECONNECT {
                    communication.status.update(|status| {
                        status.run = Run::Reconnecting;
                    });
                    borg.wait()?;
                    std::thread::sleep(crate::BORG_DELAY_RECONNECT);
                    return self.create_internal(communication, retries + 1);
                } else {
                    communication.status.update(move |status| {
                        if let Some(history) = status.message_history.last_mut() {
                            if history.0.len() < Status::MESSAGE_HISTORY_LENGTH {
                                history.0.push(msg.clone());
                            } else if history.1.len() < Status::MESSAGE_HISTORY_LENGTH {
                                history.1.push(msg.clone());
                            } else {
                                if let Some(position) =
                                    history.1.iter().position(|x| x.level() < msg.level())
                                {
                                    history.1.remove(position);
                                } else {
                                    history.1.remove(0);
                                }
                                history.1.push(msg.clone());
                            }
                        }
                    });
                }
            }

            line.clear();
        }

        let output = borg.wait_with_output()?;
        let exit_status = output.status;
        debug!("borg::create exited with {:?}", exit_status.code());

        let stats = serde_json::from_slice(&output.stdout);
        info!("Stats: {:#?}", stats);

        if exit_status.success()
            || communication
                .status
                .load()
                .last_combined_message_history()
                .max_log_level()
                < Some(LogLevel::Error)
        {
            Ok(stats?)
        } else if let Ok(err) =
            Error::try_from(communication.status.load().last_combined_message_history())
        {
            Err(err)
        } else {
            Err(error::ReturnCodeError::new(exit_status.code()).into())
        }
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

    fn list(&self, last: u64) -> Result<Vec<Archive>> {
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
