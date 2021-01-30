use super::{Borg, BorgRunConfig, Error, Result};
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::os::unix::io::IntoRawFd;
use std::process::{Command, Stdio};
use zeroize::Zeroizing;

use super::error::*;
use super::msg::*;

#[derive(Default)]
pub struct BorgCall {
    command: Option<String>,
    options: Vec<String>,
    envs: std::collections::BTreeMap<String, String>,
    pub positional: Vec<String>,
}

pub fn check_stderr(output: &std::process::Output) -> Result<()> {
    let mut errors = Vec::new();
    for line in String::from_utf8_lossy(&output.stderr).lines() {
        let msg = check_line(line);
        if msg
            .message()
            .contains("By default repositories initialized with this version will produce security")
            || msg
                .message()
                .contains("IMPORTANT: you will need both KEY AND PASSPHRASE to access this repo!")
        {
            info!("Hiding this message");
        } else {
            errors.push(msg);
        }
    }

    if output.status.success() {
        Ok(())
    } else if errors.is_empty() {
        error!(
            "borg return code is '{:?}' but couldn't find an error message",
            output.status.code()
        );
        Err(ReturnCodeErr::new(output.status.code()).into())
    } else {
        Err(LogMessageCollection::new(errors).into())
    }
}

pub fn check_line(line: &str) -> LogMessageEnum {
    if let Ok(mut msg @ LogMessage { .. }) = serde_json::from_str(line) {
        if msg.msgid == MsgId::Undefined {
            let msgid_helper_parsed: std::result::Result<MsgIdHelper, _> =
                serde_json::from_str(line);
            if let Ok(msgid_helper) = msgid_helper_parsed {
                msg.msgid = MsgId::Other(msgid_helper.msgid);
            }
        }
        info!("LogMessage {:?}", msg);

        LogMessageEnum::ParsedErr(msg)
    } else {
        error!("Parse error {}", line);
        LogMessageEnum::UnparsableErr(line.to_string())
    }
}

impl BorgCall {
    pub fn new(command: &str) -> Self {
        Self {
            command: Some(command.to_string()),
            options: vec!["--rsh".into(), "ssh -o BatchMode=yes".into()],
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
        <L as std::iter::IntoIterator>::Item: ToString,
    {
        for option in options {
            self.options.push(option.to_string());
        }

        self
    }

    pub fn add_positional<A: ToString>(&mut self, pos_arg: &A) -> &mut Self {
        self.positional.push(pos_arg.to_string());
        self
    }

    pub fn add_include_exclude(&mut self, borg: &Borg) -> &mut Self {
        for exclude in &borg.config.exclude_dirs_internal() {
            self.add_options(vec![format!(
                "--exclude={}:{}",
                exclude.selector(),
                exclude.pattern()
            )]);
        }

        self.positional.extend(
            borg.config
                .include_dirs()
                .iter()
                .map(|d| d.to_string_lossy().to_string()),
        );

        self
    }

    pub fn add_archive(&mut self, borg: &Borg) -> &mut Self {
        let random_str = glib::uuid_string_random();
        let arg = format!(
            "{}::{}",
            &borg.config.repo,
            random_str.get(..8).unwrap_or(&random_str)
        );
        if let Some(first) = self.positional.first_mut() {
            *first = arg;
        } else {
            self.add_positional(&arg);
        }

        self
    }

    pub fn add_password<T: BorgRunConfig>(&mut self, borg: &T) -> Result<&mut Self> {
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

        self.envs.insert(
            "BORG_PASSPHRASE_FD".to_string(),
            pipe_reader.into_raw_fd().to_string(),
        );

        if let Some(ref password) = borg.get_password() {
            debug!("Using password enforced by explicitly passed password");
            pipe_writer.write_all(password)?;
        } else if borg.is_encrypted() {
            debug!("Config says the backup is encrypted");
            if let Some(config_id) = borg.get_config_id() {
                let password: Zeroizing<Vec<u8>> =
                    secret_service::SecretService::new(secret_service::EncryptionType::Dh)?
                        .search_items(vec![
                            ("backup_id", config_id.as_str()),
                            ("program", env!("CARGO_PKG_NAME")),
                        ])?
                        .get(0)
                        .ok_or(Error::PasswordMissing)?
                        .get_secret()?
                        .into();
                pipe_writer.write_all(&password)?;
            } else {
                // TODO when is this happening?
                return Err(Error::PasswordMissing);
            }
        } else {
            trace!("Config says no encryption. Writing empty passsword.");
            pipe_writer.write_all(b"")?;
        }

        drop(pipe_writer);

        Ok(self)
    }

    pub fn add_basics<T: BorgRunConfig>(&mut self, borg: &T) -> Result<&mut Self> {
        self.add_options(&["--log-json"]);

        if self.positional.is_empty() {
            self.add_positional(&borg.get_repo().to_string());
        }

        self.add_password(borg)?;

        self.add_options(
            &borg
                .get_repo()
                .get_settings()
                .and_then(|x| x.command_line_args)
                .unwrap_or_default(),
        );

        Ok(self)
    }

    pub fn args(&self) -> Vec<String> {
        let mut args: Vec<String> = self.command.clone().into_iter().collect();
        args.extend(self.options.clone());
        args.push("--".to_string());
        args.extend(self.positional.clone());

        args
    }

    pub fn cmd(&self) -> Command {
        let mut cmd = Command::new("borg");

        cmd.args(self.args())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .envs(self.envs.clone().into_iter());

        cmd
    }

    pub fn spawn(&self) -> std::io::Result<std::process::Child> {
        info!("Running borg: {:#?}\nenv: {:#?}", &self.args(), &self.envs);
        self.cmd().spawn()
    }

    pub fn output(&self) -> std::io::Result<std::process::Output> {
        info!("Running borg: {:#?}\nenv: {:#?}", &self.args(), &self.envs);
        self.cmd().output()
    }
}
