use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::ffi::OsString;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::UnixStream;
use std::time::Duration;

use async_process;
use async_process::{ChildStderr, ChildStdin};
use futures_util::FutureExt;
use serde::{Deserialize, Serialize};
use smol::io::BufReader;
use smol::prelude::*;

use super::communication::*;
use super::error::*;
use super::prelude::*;
use super::status::*;
use super::{BorgRunConfig, Command, Error, Result, Task, USER_INTERACTION_TIME, log_json, utils};
use crate::config;

/// Return raw stdout from `BorgCall` instead JSON decoding it
#[derive(Debug, Serialize, Deserialize)]
pub struct RawOutput {
    pub output: Vec<u8>,
}

/// Manages calling borg
///
/// Spawning one `BorgCall`` can involve multiple successive `BorgProcess`es to
/// be spawned to handle reconnects.
#[derive(Default)]
pub struct BorgCall {
    command: Option<OsString>,
    sub_commands: Vec<OsString>,
    options: Vec<OsString>,
    envs: std::collections::BTreeMap<String, String>,
    pub positional: Vec<OsString>,
    password: config::Password,
}

impl std::fmt::Debug for BorgCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut filtered_envs = self.envs.clone();

        // TODO: It would be better if this passphrase could be sent via passfifo too
        filtered_envs
            .entry("BORG_NEW_PASSPHRASE".to_string())
            .and_modify(|e| *e = "***".to_string());

        f.debug_struct("BorgCall")
            .field("args", &self.args())
            .field("envs", &filtered_envs)
            .finish()
    }
}

pub struct Process<T> {
    pub result: smol::Task<Result<T>>,
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

    pub fn add_sub_command(&mut self, sub_command: impl Into<OsString>) -> &mut Self {
        self.sub_commands.push(sub_command.into());

        self
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
        match borg.password() {
            Some(ref password) => {
                tracing::debug!("Using password enforced by explicitly passed password");
                self.password = password.clone();
            }
            _ => {
                if borg.is_encrypted() {
                    tracing::debug!("Config says the backup is encrypted");
                    match borg.try_config() {
                        Some(config) => {
                            let password = match self.get_password_keyring(&config.repo_id).await {
                                // keyring is available and has the password
                                Ok(password) => password,
                                // keyring is available but doesn't have the password
                                Err(
                                    err @ Error::PasswordMissing {
                                        keyring_error: None,
                                    },
                                ) => Err(err)?,
                                // keyring unavailable
                                Err(err) => {
                                    tracing::warn!(
                                        "Error using keyring, using in-memory password store. Keyring error: '{err:?}'"
                                    );

                                    // Use the in-memory password store
                                    crate::globals::MEMORY_PASSWORD_STORE
                                        .load_password(&config)
                                        .ok_or(Error::PasswordMissing {
                                            keyring_error: Some(err.to_string()),
                                        })?
                                }
                            };

                            self.password = password;
                        }
                        _ => {
                            // TODO when is this happening?
                            return Err(Error::PasswordMissing {
                                keyring_error: None,
                            });
                        }
                    }
                } else {
                    tracing::trace!("Config says no encryption. Writing empty password.");
                    self.password = config::Password::default();
                }
            }
        }

        Ok(self)
    }

    async fn get_password_keyring(&self, repo_id: &super::RepoId) -> Result<config::Password> {
        Ok(config::Password::from(
            oo7::Keyring::new()
                .await?
                .search_items(&HashMap::from([("repo-id", repo_id.as_str())]))
                .await?
                .first()
                .ok_or(Error::PasswordMissing {
                    keyring_error: None,
                })?
                .secret()
                .await?,
        ))
    }

    fn stream_password(&self, command: &mut async_process::Command) -> Result<UnixStream> {
        // Password pipe
        let (pipe_reader, mut pipe_writer) = std::os::unix::net::UnixStream::pair()?;

        // Allow pipe to be passed to borg
        let mut flags = nix::fcntl::FdFlag::from_bits_truncate(nix::fcntl::fcntl(
            &pipe_reader,
            nix::fcntl::FcntlArg::F_GETFD,
        )?);

        flags.remove(nix::fcntl::FdFlag::FD_CLOEXEC);
        nix::fcntl::fcntl(&pipe_reader, nix::fcntl::FcntlArg::F_SETFD(flags))?;

        // We drop the pipe_writer here, so this end will be closed when this function
        // returns
        pipe_writer.write_all(self.password.as_bytes())?;

        let fd = pipe_reader.as_raw_fd();

        command.env("BORG_PASSPHRASE_FD", fd.to_string());

        // Keep the fd around to only close it after the process is finished
        Ok(pipe_reader)
    }

    pub fn add_basics_without_password<T: BorgRunConfig>(&mut self, borg: &T) -> &mut Self {
        self.add_options(&["--log-json"]);

        if self.positional.is_empty() {
            self.add_positional(borg.repo().to_string());
        }

        self.add_options(
            borg.repo()
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

    fn args(&self) -> Vec<OsString> {
        let mut args: Vec<OsString> = self.command.clone().into_iter().collect();
        args.extend(self.sub_commands.clone());
        args.extend(self.options.clone());
        args.push("--".into());
        args.extend(self.positional.clone());

        args
    }

    pub(super) fn command(&self) -> Result<(async_process::Command, UnixStream)> {
        let mut cmd = async_process::Command::new("borg");

        let unix_stream = self.stream_password(&mut cmd)?;

        cmd.args(self.args())
            .stderr(async_process::Stdio::piped())
            .stdout(async_process::Stdio::piped())
            .stdin(async_process::Stdio::piped())
            .envs(self.envs.clone());

        Ok((cmd, unix_stream))
    }

    /// Calls the borg command and returns the output
    ///
    /// The output is JSON decoded already if not using `RawOutput`.
    pub async fn output_generic<
        S: std::fmt::Debug + serde::de::DeserializeOwned + Send + Sync + 'static,
    >(
        &self,
    ) -> Result<S> {
        let communication = Communication::<super::task::Generic>::default();
        let sender = communication.new_sender();

        let managed_process = BorgProcess::new(self, communication, sender).await?;
        managed_process.spawn::<S>().await
    }

    /// Calls the borg command with `Communication`
    ///
    /// Handles disconnects.
    pub async fn output<
        T: Task,
        S: std::fmt::Debug + serde::de::DeserializeOwned + Send + Sync + 'static,
    >(
        self,
        communication: &super::Communication<T>,
    ) -> Result<S> {
        self.handle_disconnect(communication.clone()).await
    }

    /// Spawn a borg task, parsing the output as `S`
    ///
    /// Returns immedialetly running the task in the background. Handles
    /// disconnects.
    pub fn spawn_background<
        T: Task,
        S: std::fmt::Debug + serde::de::DeserializeOwned + Send + Sync + 'static,
    >(
        self,
        communication: &super::Communication<T>,
    ) -> Result<Process<S>> {
        let result = smol::spawn(self.handle_disconnect(communication.clone()));

        Ok(Process { result })
    }

    /// Spawns the command with disconnect handling
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
        let started_instant = std::time::Instant::now();

        let mut retries = 0;
        let mut retried = false;

        loop {
            // track separate history for each run
            communication.general_info.update(|x| {
                x.message_history.push(Default::default());
            });

            let managed_process =
                BorgProcess::new(&self, communication.clone(), sender.clone()).await?;
            let result = managed_process.spawn().await;

            match &result {
                Err(Error::Failed(failure)) if failure.is_connection_error() => {
                    if !communication.general_info.load().is_schedule
                        && std::time::Instant::now().duration_since(started_instant)
                            < USER_INTERACTION_TIME
                    {
                        // Don't reconnect when manual backups fail right at the beginning. This is
                        // most likely a permanent problem.
                        return result;
                    }

                    if !retried {
                        tracing::debug!("First disconnect for this task");
                        retried = true;
                        self.add_options(&[
                            "--lock-wait",
                            &super::LOCK_WAIT_RECONNECT.as_secs().to_string(),
                        ]);
                    }

                    if matches!(communication.status(), RunStatus::Running) {
                        tracing::debug!("Starting reconnect attempts");
                        retries = 0;
                        communication.set_status(RunStatus::Reconnecting(super::DELAY_RECONNECT));
                    }

                    if retries < super::MAX_RECONNECT {
                        retries += 1;
                        tracing::debug!("Reconnect attempt number {}", retries);

                        let start_time = std::time::Instant::now();
                        while start_time.elapsed() < super::DELAY_RECONNECT {
                            if let Instruction::Abort(ref reason) =
                                **communication.instruction.load()
                            {
                                return Err(Error::Aborted(reason.clone()));
                            }

                            communication.set_status(RunStatus::Reconnecting(
                                super::DELAY_RECONNECT
                                    .checked_sub(start_time.elapsed())
                                    .unwrap_or(Duration::ZERO),
                            ));

                            smol::Timer::after(Duration::from_millis(100)).await;
                        }

                        communication.set_status(RunStatus::Init);
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
}

/// Represents an actual process
struct BorgProcess<'a, T: Task> {
    _call: &'a BorgCall,
    communication: super::Communication<T>,
    sender: Sender<T>,
    command: async_process::Command,
    // Keep the stream around until process is finished
    _password_stream: UnixStream,
}

impl<'a, T: Task> BorgProcess<'a, T> {
    /// Prepare a new porg process
    async fn new(
        call: &'a BorgCall,
        communication: super::Communication<T>,
        sender: Sender<T>,
    ) -> Result<BorgProcess<'a, T>> {
        let (command, password_stream) = call.command()?;

        Ok(Self {
            _call: call,
            communication,
            sender,
            command,
            _password_stream: password_stream,
        })
    }

    /// Set the CPU scheduler priority of a process
    fn set_scheduler_priority(pid: u32, priority: i32) {
        tracing::debug!("Setting scheduler priority to {}", priority);
        let result = unsafe { nix::libc::setpriority(nix::libc::PRIO_PROCESS, pid, priority) };
        if result != 0 {
            tracing::warn!("Failed to set scheduler priority: {}", result);
        }
    }

    /// Run the borg process
    async fn spawn<S: std::fmt::Debug + serde::de::DeserializeOwned + 'static>(
        mut self,
    ) -> Result<S> {
        tracing::info!("Running managed borg command: {:?}", self.command);
        tracing::debug!("Command details: {:#?}", self.command);

        let mut process = self.command.spawn()?;

        // Set CPU scheduler priority to 10 (medium-low)
        // This prevents backup operations from straining the system resources
        Self::set_scheduler_priority(process.id(), 10);

        let stderr = smol::io::BufReader::new(
            process
                .stderr
                .take()
                .ok_or_else(|| String::from("Failed to get stderr."))?,
        );

        let mut stdout = smol::io::BufReader::new(
            process
                .stdout
                .take()
                .ok_or_else(|| String::from("Failed to get stdout."))?,
        );

        let stdin = process
            .stdin
            .take()
            .ok_or_else(|| String::from("Failed to get stdin"))?;

        let mut stdout_content = Vec::new();

        // Handle stderr and collect stdout to avoid pipe stall
        let (return_message, _) = futures_util::join!(
            self.handle_stderr(stderr, stdin, process.id()),
            stdout.read_to_end(&mut stdout_content)
        );

        let status: async_process::ExitStatus = process.status().await?;
        tracing::debug!("Process terminated");

        // Return with potential errors from stderr handling
        return_message?;

        // Don't JSON decode some return types
        let result: Result<Box<S>> = if TypeId::of::<S>() == TypeId::of::<()>() {
            // Intrepret `()` return type as ignoring stdout completely
            Ok((Box::new(()) as Box<dyn Any>).downcast().unwrap())
        } else if TypeId::of::<S>() == TypeId::of::<RawOutput>() {
            // Interpret `RawOutput` as just returning the bytes instead of JSON decoding
            Ok((Box::new(RawOutput {
                output: stdout_content,
            }) as Box<dyn Any>)
                .downcast()
                .unwrap())
        } else {
            // JSON decode for all other return types
            serde_json::from_slice(&stdout_content).map_err(Into::into)
        };

        let max_log_level = self
            .communication
            .general_info
            .load()
            .last_combined_message_history()
            .max_log_level();

        tracing::debug!("Return code: {:?}", status.code());
        tracing::debug!("Maximum log level entry: {:?}", max_log_level);

        // borg also returns >0 for warnings, therefore check messages
        if status.success() || max_log_level < Some(log_json::LogLevel::Error) {
            Ok(*result?)
        } else {
            match Error::try_from(
                self.communication
                    .general_info
                    .load()
                    .last_combined_message_history(),
            ) {
                Ok(err) => Err(err),
                _ => Err(ReturnCodeError::new(status.code()).into()),
            }
        }
    }

    /// Handle the stderr output and `Communication` signals while the process
    /// is running
    async fn handle_stderr(
        &self,
        mut stderr: BufReader<ChildStderr>,
        mut stdin: ChildStdin,
        pid: u32,
    ) -> Result<()> {
        let mut return_message = Ok(());
        let mut unresponsive = Duration::ZERO;
        let mut stderr_line = String::new();

        loop {
            // react to instructions before potentially listening for messages again

            match &**self.communication.instruction.load() {
                Instruction::Abort(reason) => {
                    self.communication.set_status(RunStatus::Stopping);

                    tracing::debug!("Sending SIGINT to borg process");
                    nix::sys::signal::kill(
                        nix::unistd::Pid::from_raw(pid.try_into().unwrap()),
                        nix::sys::signal::Signal::SIGINT,
                    )?;

                    // This is needed if there is a pending question where SIGINT won't work
                    // <https://github.com/borgbackup/borg/issues/8521>
                    tracing::debug!("Sending default answer borg process");
                    stdin.write_all("\n".to_string().as_bytes()).await?;

                    // Do not return immediately to get further progress information
                    // and be able to send signal again.
                    return_message = Err(Error::Aborted(reason.clone()));
                    self.communication.set_instruction(Instruction::Nothing);
                }
                Instruction::Response(response) => {
                    tracing::warn!("Sending response “{response}” to borg process");
                    stdin.write_all(format!("{response}\n").as_bytes()).await?;
                    self.communication.set_instruction(Instruction::Nothing);
                }
                Instruction::Nothing => {}
            }

            stderr_line.clear();
            // Listen to stderr with timeout to also handle instructions in-between
            let stderr_result = futures_util::select!(
                _ = futures_util::FutureExt::fuse(smol::Timer::after(super::MESSAGE_POLL_TIMEOUT)) => Err(()),
                res = stderr.read_line(&mut stderr_line).fuse() => Ok(res),
            );

            match stderr_result {
                // Nothing new to read
                Err(()) => {
                    unresponsive += super::MESSAGE_POLL_TIMEOUT;
                    if unresponsive > super::STALL_THRESHOLD
                        && !matches!(self.communication.status(), RunStatus::Reconnecting(_))
                    {
                        self.communication.set_status(RunStatus::Stalled);
                    }
                    continue;
                }
                Ok(Err(err)) => return Err(err.into()),
                // end of stream
                Ok(Ok(0)) => return return_message,
                // one line read
                Ok(Ok(_)) => {
                    unresponsive = Duration::ZERO;

                    tracing::trace!("borg output: {}", stderr_line);

                    let msg =
                        if let Ok(msg) = serde_json::from_str::<log_json::Progress>(&stderr_line) {
                            if !matches!(self.communication.status(), RunStatus::Running) {
                                self.communication.set_status(RunStatus::Running);
                            }
                            log_json::Output::Progress(msg)
                        } else {
                            let msg = utils::check_line(&stderr_line);
                            if msg.is_ignored() {
                                continue;
                            }

                            msg.log(&T::name());

                            self.communication.general_info.update(|status| {
                                status.add_message(&msg);
                            });

                            log_json::Output::LogEntry(msg)
                        };

                    self.sender.send(msg.clone()).await?;
                }
            }
        }
    }
}
