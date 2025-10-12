use std::collections::HashMap;

use super::error::*;
use super::prelude::*;
use crate::borg::Outcome;
use crate::config::UserScriptKind;

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ShellVariable {
    ConfigId,
    ConfigName,
    RepoId,
    Url,
    IsSchedule,

    // post backup script only
    Outcome,
    ResultMsg,

    // completed backups only
    ArchiveId,
    ArchiveName,
    BytesTotal,
    BytesCompressed,
    BytesUnique,
    FilesCount,
}

impl ShellVariable {
    pub fn all() -> &'static [ShellVariable] {
        static ALL: [ShellVariable; 13] = [
            ShellVariable::ConfigId,
            ShellVariable::ConfigName,
            ShellVariable::RepoId,
            ShellVariable::Url,
            ShellVariable::IsSchedule,
            ShellVariable::Outcome,
            ShellVariable::ArchiveId,
            ShellVariable::ArchiveName,
            ShellVariable::ResultMsg,
            ShellVariable::BytesTotal,
            ShellVariable::BytesCompressed,
            ShellVariable::BytesUnique,
            ShellVariable::FilesCount,
        ];

        &ALL
    }

    pub fn explanation_string_markup() -> String {
        let intro = gettext("The following variables are available:");
        let mut all = String::new();
        let post_backup_msg = gettext("Only available for the post-backup command:");
        let mut post_backup = String::new();
        let completed_msg = gettext("Only available for completed backups:");
        let mut completed = String::new();

        for var in Self::all() {
            let new = format!(
                "\n<span line_height='0.3'> </span>\n<tt><b>${}</b></tt>: {}",
                var.name(),
                var.description()
            );

            if var.is_completed_only() {
                completed += &new;
            } else if var.is_post_backup_only() {
                post_backup += &new;
            } else {
                all += &new;
            }
        }

        format!("{intro}{all}\n\n{post_backup_msg}{post_backup}\n\n{completed_msg}{completed}")
    }

    pub fn name(&self) -> &'static str {
        match self {
            ShellVariable::ConfigId => "CONFIG_ID",
            ShellVariable::ConfigName => "CONFIG_NAME",
            ShellVariable::RepoId => "REPO_ID",
            ShellVariable::Url => "URL",
            ShellVariable::IsSchedule => "IS_SCHEDULE",
            ShellVariable::Outcome => "OUTCOME",
            ShellVariable::ResultMsg => "RESULT_MSG",
            ShellVariable::ArchiveId => "ARCHIVE_ID",
            ShellVariable::ArchiveName => "ARCHIVE_NAME",
            ShellVariable::BytesTotal => "BYTES_TOTAL",
            ShellVariable::BytesCompressed => "BYTES_COMPRESSED",
            ShellVariable::BytesUnique => "BYTES_UNIQUE",
            ShellVariable::FilesCount => "FILES_COUNT",
        }
    }

    pub fn description(&self) -> String {
        match self {
            ShellVariable::ConfigId => gettext("ID of the backup configuration"),
            ShellVariable::ConfigName => gettext("Title of the backup configuration"),
            ShellVariable::RepoId => gettext("Repository ID of the borg repository"),
            ShellVariable::Url => gettext("The full URL passed to borgbackup"),
            ShellVariable::IsSchedule => {
                // Translators: Do not translate '0' and '1' here, this is documentation
                // for possible variable values
                gettext("0: manual backup, 1: started from a schedule")
            }
            ShellVariable::Outcome => {
                // Translators: String uses pango markup. Do not translate capslocked words
                // they are values for a variable.
                gettext("Either “<tt>COMPLETED</tt>”, “<tt>ABORTED</tt>” or “<tt>FAILED</tt>”")
            }
            ShellVariable::ResultMsg => gettext("An error/warning message"),
            ShellVariable::ArchiveId => gettext("The ID of the created backup archive"),
            ShellVariable::ArchiveName => gettext("The name of the created backup archive"),
            ShellVariable::BytesTotal => {
                gettext("The total amount of bytes referenced by this archive")
            }
            ShellVariable::BytesCompressed => {
                gettext("The compressed size of all data references by this archive")
            }
            ShellVariable::BytesUnique => {
                gettext("The deduplicated amount of bytes for this archive")
            }
            ShellVariable::FilesCount => gettext("The amount of files saved in this archive"),
        }
    }

    pub fn is_post_backup_only(&self) -> bool {
        matches!(
            self,
            ShellVariable::Outcome
                | ShellVariable::ResultMsg
                | ShellVariable::ArchiveId
                | ShellVariable::ArchiveName
                | ShellVariable::BytesTotal
                | ShellVariable::BytesCompressed
                | ShellVariable::BytesUnique
                | ShellVariable::FilesCount,
        )
    }

    pub fn is_completed_only(&self) -> bool {
        matches!(
            self,
            ShellVariable::ArchiveId
                | ShellVariable::ArchiveName
                | ShellVariable::BytesTotal
                | ShellVariable::BytesCompressed
                | ShellVariable::BytesUnique
                | ShellVariable::FilesCount
        )
    }
}

/// Create an environment variable dict with variables for the current backup
pub fn script_env_pre(
    config: &crate::config::Backup,
    is_schedule: bool,
) -> HashMap<ShellVariable, String> {
    let mut env = HashMap::new();
    env.insert(ShellVariable::ConfigId, config.id.to_string());
    env.insert(ShellVariable::ConfigName, config.title());
    env.insert(ShellVariable::RepoId, config.repo_id.as_str().to_string());
    env.insert(ShellVariable::Url, config.repo.to_string());
    env.insert(
        ShellVariable::IsSchedule,
        (is_schedule as usize).to_string(),
    );

    env
}

/// Create an environment variable dict with variables from the previous backup
pub fn script_env_post(
    config: &crate::config::Backup,
    is_schedule: bool,
    run_info: &crate::config::history::RunInfo,
) -> HashMap<ShellVariable, String> {
    let mut env = script_env_pre(config, is_schedule);

    env.insert(
        ShellVariable::Outcome,
        match &run_info.outcome {
            crate::borg::Outcome::Completed { .. } => "COMPLETED",
            crate::borg::Outcome::Aborted(_) => "ABORTED",
            crate::borg::Outcome::Failed(_) => "FAILED",
        }
        .to_string(),
    );

    let messages: Vec<String> = run_info.messages.iter().map(|e| e.message()).collect();

    env.insert(ShellVariable::ResultMsg, messages.join("\n"));

    if let Outcome::Completed { stats } = &run_info.outcome {
        env.insert(
            ShellVariable::ArchiveId,
            stats.archive.id.as_str().to_string(),
        );
        env.insert(
            ShellVariable::ArchiveName,
            stats.archive.name.as_str().to_string(),
        );
        env.insert(
            ShellVariable::BytesTotal,
            stats.archive.stats.original_size.to_string(),
        );
        env.insert(
            ShellVariable::BytesCompressed,
            stats.archive.stats.compressed_size.to_string(),
        );
        env.insert(
            ShellVariable::BytesUnique,
            stats.archive.stats.deduplicated_size.to_string(),
        );
        env.insert(
            ShellVariable::FilesCount,
            stats.archive.stats.nfiles.to_string(),
        );
    }

    env
}

/// Run a script on the flatpak host
///
/// Will be executed with `flatpak-spawn` and `bash -c`
pub async fn run_script(
    command: &str,
    env: HashMap<ShellVariable, String>,
    kind: UserScriptKind,
    communication: super::Communication<super::task::UserScript>,
) -> Result<()> {
    let envs: HashMap<&str, &str> = env.iter().map(|(k, v)| (k.name(), v.as_str())).collect();

    debug!(
        "Running shell script:\nbash -c \"{}\"\nenv: {:#?}",
        command, envs
    );

    let mut cmd = if *APP_IS_SANDBOXED {
        let mut cmd = async_process::Command::new("flatpak-spawn");

        // Don't remove the entire env, flatpak-spawn needs some of it
        // Prevents debug logging to influence flatpak-spawn output
        cmd.env_remove("GTK_DEBUG");
        cmd.env_remove("G_LOG_DOMAIN");
        cmd.env_remove("G_MESSAGES_DEBUG");

        for (name, value) in &envs {
            cmd.arg(format!("--env={name}={value}"));
        }

        // `bash -c` will reset most environment variables to default
        cmd.args(["--host", "bash", "-c", command]);
        cmd
    } else {
        let mut cmd = async_process::Command::new("bash");

        cmd.envs(envs);

        cmd.args(["-c", command]);
        cmd
    };

    let output = cmd
        .output_with_communication(communication)
        .await
        .map_err(|e| {
            if let Error::Aborted(_) = e {
                e
            } else {
                match kind {
                    UserScriptKind::PreBackup => Error::from(gettextf(
                        "The pre-backup command configured in preferences failed to run.\n{}",
                        &[&format!("{:?}", e)],
                    )),
                    UserScriptKind::PostBackup => Error::from(gettextf(
                        "The post-backup command configured in preferences failed to run.\n{}",
                        &[&format!("{:?}", e)],
                    )),
                }
            }
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let return_code = output.status.code().map_or(0, |c| c as u32);

    debug!("Shell script finished with code: {}", return_code);

    if !stdout.trim().is_empty() {
        debug!("stdout:\n{}", &stdout.trim());
    }

    if !stderr.trim().is_empty() {
        debug!("stderr:\n{}", &stderr.trim());
    }

    if return_code == 0 {
        Ok(())
    } else {
        let mut msg = match kind {
            UserScriptKind::PreBackup => gettextf(
                "The pre-backup command configured in preferences returned a failure code: {}",
                &[&return_code.to_string()],
            ),
            UserScriptKind::PostBackup => gettextf(
                "The post-backup command configured in preferences returned a failure code: {}",
                &[&return_code.to_string()],
            ),
        };

        if !stderr.is_empty() {
            msg += "\n\n";
            msg += stderr.trim();
        }

        Err(Error::from(msg))
    }
}
