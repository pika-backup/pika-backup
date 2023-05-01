use std::collections::HashMap;

use super::error::*;
use super::prelude::*;
use crate::borg::Outcome;

#[derive(Clone)]
pub enum UserScriptKind {
    PreBackup,
    PostBackup,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ShellVariable {
    ConfigId,
    RepoId,
    Url,
    IsSchedule,

    // post backup script only
    Outcome,
    ResultMsg,
    ArchiveId,
    ArchiveName,
    BytesTotal,
    BytesCompressed,
    BytesUnique,
    FilesCount,
}

impl ShellVariable {
    pub fn all() -> &'static [ShellVariable] {
        static ALL: [ShellVariable; 12] = [
            ShellVariable::ConfigId,
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
        let success_msg = gettext("Only available for completed backups:");
        let mut success = String::new();

        for var in Self::all() {
            let new = format!(
                "\n<tt><span fgcolor=\"#1a5fb4\">${}</span></tt>: {}",
                var.name(),
                var.description()
            );

            if var.is_success_only() {
                success += &new;
            } else if var.is_post_backup_only() {
                post_backup += &new;
            } else {
                all += &new;
            }
        }

        format!("{intro}\n{all}\n\n{post_backup_msg}\n{post_backup}\n\n{success_msg}\n{success}")
    }

    pub fn name(&self) -> &'static str {
        match self {
            ShellVariable::ConfigId => "CONFIG_ID",
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
            ShellVariable::ConfigId => gettext("ID of the backup config"),
            ShellVariable::RepoId => gettext("Repository ID of the borg repository"),
            ShellVariable::Url => gettext("The full url passed to borgbackup"),
            ShellVariable::IsSchedule => gettext("0: manual backup, 1: started from a schedule"),
            ShellVariable::Outcome => gettext("Either COMPLETED, ABORTED or FAILED"),
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

    pub fn is_success_only(&self) -> bool {
        matches!(self, |ShellVariable::ArchiveId| ShellVariable::ArchiveName
            | ShellVariable::BytesTotal
            | ShellVariable::BytesCompressed
            | ShellVariable::BytesUnique
            | ShellVariable::FilesCount)
    }
}

pub fn script_env_pre(
    config: &crate::config::Backup,
    is_schedule: bool,
) -> HashMap<ShellVariable, String> {
    let mut env = HashMap::new();
    env.insert(ShellVariable::ConfigId, config.id.to_string());
    env.insert(ShellVariable::RepoId, config.repo_id.as_str().to_string());
    env.insert(ShellVariable::Url, config.repo.to_string());
    env.insert(
        ShellVariable::IsSchedule,
        if is_schedule { "1" } else { "0" }.to_string(),
    );

    env
}

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
        let mut cmd = async_std::process::Command::new("flatpak-spawn");
        cmd.args(["--host", "bash", "-c", command]);
        cmd
    } else {
        let mut cmd = async_std::process::Command::new("bash");
        cmd.args(["-c", command]);
        cmd
    };

    cmd.env_remove("GTK_DEBUG");
    cmd.env_remove("G_LOG_DOMAIN");
    cmd.env_remove("G_MESSAGES_DEBUG");
    cmd.envs(envs);

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
            msg += &stderr.trim();
        }

        Err(Error::from(msg))
    }
}
