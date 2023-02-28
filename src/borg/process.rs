use super::prelude::*;
use async_std::prelude::*;

use super::{BorgRunConfig, Command, Error, Result};

use std::any::TypeId;
use std::collections::HashMap;
use std::ffi::OsString;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::os::unix::io::IntoRawFd;
use std::process::{self, Stdio};

use std::time::Duration;

use super::communication::*;
use super::log_json;
use super::status::*;
use super::utils;
use super::Task;
use crate::config;

use async_std::process as async_process;

use super::error::*;

#[derive(Default)]
pub struct BorgCall {
    command: Option<OsString>,
    options: Vec<OsString>,
    envs: std::collections::BTreeMap<String, String>,
    pub positional: Vec<OsString>,
    password: config::Password,
}

pub struct Process<T> {
    pub result: async_std::task::JoinHandle<Result<T>>,
}

impl BorgCall {
    pub fn new(command: impl Into<OsString>) -> Self {
        Self {
            command: Some(command.into()),
            options: vec![
                "--rsh".into(),
                // Avoid hangs from ssh asking for passwords via stdin
                // https://borgbackup.readthedocs.io/en/stable/usage/notes.html#ssh-batch-mode
                "ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new".into(),
            ],
            ..Self::default()
        }
    }

    pub fn new_raw() -> Self {
        Self::default()
    }

    pub fn add_envs<L, V>(&mut self, vars: L) -> &mut Self
    where
        L: std::iter::IntoIterator<Item = (V, V)>,
        V: ToString,
    {
        for (var, value) in vars {
            self.envs.insert(var.to_string(), value.to_string());
        }

        self
    }

    pub fn add_options<L>(&mut self, options: L) -> &mut Self
    where
        L: std::iter::IntoIterator,
        <L as std::iter::IntoIterator>::Item: Into<OsString>,
    {
        for option in options {
            self.options.push(option.into());
        }

        self
    }

    pub fn add_positional(&mut self, pos_arg: impl Into<OsString>) -> &mut Self {
        self.positional.push(pos_arg.into());
        self
    }

    pub fn add_include_exclude<T: Task>(&mut self, borg: &Command<T>) -> &mut Self {
        for exclude in &borg.config.exclude_dirs_internal() {
            for rule in exclude.borg_rules() {
                match rule {
                    config::exclude::BorgRule::Pattern(pattern) => {
                        let mut arg = OsString::from("--exclude=");
                        arg.push(pattern);
                        self.add_options(vec![arg]);
                    }
                    config::exclude::BorgRule::CacheDirTag => {
                        self.add_options(vec!["--exclude-caches"]);
                    }
                }
            }
        }
        self.positional.extend(
            borg.config
                .include_dirs()
                .iter()
                .map(|d| d.clone().into_os_string()),
        );

        self
    }

    pub fn add_archive<T: Task>(&mut self, borg: &Command<T>) -> &mut Self {
        let random_str = glib::uuid_string_random();
        let arg = format!(
            "{repo}::{archive_prefix}{archive}",
            repo = borg.config.repo,
            archive_prefix = borg.config.archive_prefix,
            archive = random_str.get(..8).unwrap_or(&random_str)
        );
        if let Some(first) = self.positional.first_mut() {
            *first = arg.into();
        } else {
            self.add_positional(&arg);
        }

        self
    }

    pub async fn add_password<T: BorgRunConfig>(&mut self, borg: &T) -> Result<&mut Self> {
        if let Some(ref password) = borg.password() {
            debug!("Using password enforced by explicitly passed password");
            self.password = password.clone();
        } else if borg.is_encrypted() {
            debug!("Config says the backup is encrypted");
            if let Some(config) = borg.try_config() {
                let password = config::Password::from(
                    oo7::Keyring::new()
                        .await?
                        .search_items(HashMap::from([("repo-id", config.repo_id.as_str())]))
                        .await?
                        .first()
                        .ok_or(Error::PasswordMissing)?
                        .secret()
                        .await?,
                );

                self.password = password;
            } else {
                // TODO when is this happening?
                return Err(Error::PasswordMissing);
            }
        } else {
            trace!("Config says no encryption. Writing empty password.");
            self.password = config::Password::default();
        }

        Ok(self)
    }

    fn set_password(&self) -> Result<(String, String)> {
        // Password pipe
        let (pipe_reader, mut pipe_writer) = std::os::unix::net::UnixStream::pair()?;

        // Allow pipe to be passed to borg
        let mut flags = nix::fcntl::FdFlag::from_bits_truncate(nix::fcntl::fcntl(
            pipe_reader.as_raw_fd(),
            nix::fcntl::FcntlArg::F_GETFD,
        )?);

        flags.remove(nix::fcntl::FdFlag::FD_CLOEXEC);
        nix::fcntl::fcntl(
            pipe_reader.as_raw_fd(),
            nix::fcntl::FcntlArg::F_SETFD(flags),
        )?;

        pipe_writer.write_all(self.password.as_bytes())?;

        Ok((
            String::from("BORG_PASSPHRASE_FD"),
            pipe_reader.into_raw_fd().to_string(),
        ))
    }

    pub fn add_basics_without_password<T: BorgRunConfig>(&mut self, borg: &T) -> &mut Self {
        self.add_options(&["--log-json"]);

        if self.positional.is_empty() {
            self.add_positional(&borg.repo().to_string());
        }

        self.add_options(
            &borg
                .repo()
                .settings()
                .and_then(|x| x.command_line_args)
                .unwrap_or_default(),
        );

        self
    }

    pub async fn add_basics<T: BorgRunConfig>(&mut self, borg: &T) -> Result<&mut Self> {
        self.add_password(borg).await?;
        self.add_basics_without_password(borg);
        Ok(self)
    }

    pub fn args(&self) -> Vec<OsString> {
        let mut args: Vec<OsString> = self.command.clone().into_iter().collect();
        args.extend(self.options.clone());
        args.push("--".into());
        args.extend(self.positional.clone());

        args
    }

    pub fn cmd(&self) -> Result<process::Command> {
        let mut cmd = process::Command::new("borg");

        cmd.envs([self.set_password()?]);

        cmd.args(self.args())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .envs(self.envs.clone().into_iter());

        Ok(cmd)
    }

    pub fn output(&self) -> Result<std::process::Output> {
        info!("Running borg: {:#?}\nenv: {:#?}", &self.args(), &self.envs);
        Ok(self.cmd()?.output()?)
    }

    pub fn cmd_async(&self) -> Result<async_process::Command> {
        let mut cmd = async_process::Command::new("borg");

        cmd.envs([self.set_password()?]);

        cmd.args(self.args())
            .stderr(async_process::Stdio::piped())
            .stdout(async_process::Stdio::piped())
            .stdin(async_process::Stdio::piped())
            .envs(self.envs.clone().into_iter());

        Ok(cmd)
    }

    pub fn spawn_async(&self) -> Result<async_process::Child> {
        info!(
            "Async running borg: {:#?}\nenv: {:#?}",
            &self.args(),
            &self.envs
        );
        Ok(self.cmd_async()?.spawn()?)
    }

    pub fn spawn_async_managed<
        T: Task,
        S: std::fmt::Debug + serde::de::DeserializeOwned + Send + 'static,
    >(
        self,
        communication: super::Communication<T>,
    ) -> Result<Process<S>> {
        let result = async_std::task::spawn(self.handle_disconnect(communication));

        Ok(Process { result })
    }

    async fn handle_disconnect<
        T: Task,
        S: std::fmt::Debug + serde::de::DeserializeOwned + 'static,
    >(
        mut self,
        communication: super::Communication<T>,
    ) -> Result<S> {
        communication.general_info.update(move |status| {
            status.started = Some(chrono::Local::now());
        });
        let sender = communication.new_sender();

        let mut retries = 0;
        let mut retried = false;

        loop {
            // track separate history for each run
            communication.general_info.update(|x| {
                x.message_history.push(Default::default());
            });
            let result = self.managed_process(communication.clone(), &sender).await;
            match &result {
                Err(Error::Failed(ref failure)) if failure.is_connection_error() => {
                    if !retried {
                        debug!("First disconnect for this task");
                        retried = true;
                        self.add_options(&[
                            "--lock-wait",
                            &super::LOCK_WAIT_RECONNECT.as_secs().to_string(),
                        ]);
                    }

                    if !matches!(communication.status(), Run::Reconnecting) {
                        debug!("Starting reconnect attempts");
                        retries = 0;
                        communication.set_status(Run::Reconnecting);
                    }

                    if retries < super::MAX_RECONNECT {
                        retries += 1;
                        debug!("Reconnect attempt number {}", retries);
                        std::thread::sleep(super::DELAY_RECONNECT);
                        continue;
                    } else {
                        return result;
                    }
                }
                _ => {
                    return result;
                }
            }
        }
    }

    async fn managed_process<
        T: Task,
        S: std::fmt::Debug + serde::de::DeserializeOwned + 'static,
    >(
        &self,
        communication: super::Communication<T>,
        sender: &Sender<T>,
    ) -> Result<S> {
        let mut return_message = None;
        let mut line = String::new();
        let mut process = self.spawn_async()?;
        let mut reader = async_std::io::BufReader::new(
            process
                .stderr
                .take()
                .ok_or_else(|| String::from("Failed to get stderr."))?,
        );
        let mut writer = process
            .stdin
            .take()
            .ok_or_else(|| String::from("Failed to get stdin"))?;

        let mut unresponsive = Duration::ZERO;

        loop {
            // react to instructions before potentially listening for messages again
            match &**communication.instruction.load() {
                Instruction::Abort(ref reason) => {
                    communication.set_status(Run::Stopping);
                    debug!("Sending SIGINT to borg process");
                    nix::sys::signal::kill(
                        nix::unistd::Pid::from_raw(process.id() as i32),
                        nix::sys::signal::Signal::SIGINT,
                    )?;
                    return_message = Some(Err(Error::Aborted(reason.clone())));
                    communication.set_instruction(Instruction::Nothing);
                }
                Instruction::Response(response) => {
                    warn!("Sending response “{response}” to borg process");
                    writer.write_all(format!("{response}\n").as_bytes()).await?;
                    communication.set_instruction(Instruction::Nothing);
                }
                Instruction::Nothing => {}
            }

            line.clear();
            let read =
                async_std::io::timeout(super::MESSAGE_POLL_TIMEOUT, reader.read_line(&mut line))
                    .await;

            match read {
                // nothing new to read
                Err(err) if err.kind() == async_std::io::ErrorKind::TimedOut => {
                    unresponsive += super::MESSAGE_POLL_TIMEOUT;
                    if unresponsive > super::STALL_THRESHOLD
                        && !matches!(communication.status(), Run::Reconnecting)
                    {
                        communication.set_status(Run::Stalled);
                    }
                    continue;
                }
                Err(err) => return Err(err.into()),
                // end of stream
                Ok(0) => break,
                // one line read
                Ok(_) => {}
            }

            unresponsive = Duration::ZERO;

            trace!("borg output: {}", line);

            let msg = if let Ok(msg) = serde_json::from_str::<log_json::Progress>(&line) {
                if !matches!(communication.status(), Run::Running) {
                    communication.set_status(Run::Running);
                }
                log_json::Output::Progress(msg)
            } else {
                let msg = utils::check_line(&line);

                communication.general_info.update(|status| {
                    status.add_message(&msg);
                });
                log_json::Output::LogEntry(msg)
            };

            sender.send(msg.clone()).await?;
        }

        let output = process.output().await?;

        debug!("Process terminated");

        let result = if TypeId::of::<S>() == TypeId::of::<()>() {
            serde_json::from_slice(b"null")
        } else {
            serde_json::from_slice(&output.stdout)
        };

        let max_log_level = communication
            .general_info
            .load()
            .last_combined_message_history()
            .max_log_level();

        debug!("Return code: {:?}", output.status.code());
        debug!("Maximum log level entry: {:?}", max_log_level);

        if let Some(msg) = return_message {
            return msg;
        }

        // borg also returns >0 for warnings, therefore check messages
        if output.status.success() || max_log_level < Some(log_json::LogLevel::Error) {
            Ok(result?)
        } else if let Ok(err) = Error::try_from(
            communication
                .general_info
                .load()
                .last_combined_message_history(),
        ) {
            Err(err)
        } else {
            Err(ReturnCodeError::new(output.status.code()).into())
        }
    }
}
