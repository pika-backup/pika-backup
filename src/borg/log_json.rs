/*!
Borg output to STDERR with `--log-json` flag.
*/

use crate::prelude::*;
use std::fmt;

/// All possible output
#[derive(Clone, Debug)]
pub enum Output {
    Progress(Progress),
    LogEntry(LogEntry),
}

impl fmt::Display for Output {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Progress(progress) => write!(f, "{progress}"),
            Self::LogEntry(log_entry) => write!(f, "{log_entry}"),
        }
    }
}

pub type MsgId = super::error::Failure;

#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum Progress {
    #[serde(rename = "archive_progress")]
    Archive(ProgressArchive),
    #[serde(rename = "progress_message")]
    Message(ProgressMessage),
    #[serde(rename = "progress_percent")]
    Percent(ProgressPercent),
    #[serde(rename = "question_prompt")]
    Question(QuestionPrompt),
    #[serde(rename = "question_accepted_false")]
    QuestionAcceptedFalse,
    #[serde(rename = "question_accepted_true")]
    QuestionAcceptedTrue,
}

impl fmt::Display for Progress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Archive(archive) => write!(f, "{archive}"),
            Self::Message(message) => write!(f, "{message}"),
            Self::Percent(percent) => write!(f, "{percent}"),
            Self::Question(question) => write!(f, "{question}"),
            Self::QuestionAcceptedFalse => {
                write!(f, "{}", gettext("Backup will be aborted."))
            }
            Self::QuestionAcceptedTrue => {
                write!(f, "{}", gettext("Backup will continue."))
            }
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct ProgressArchive {
    #[serde(default)]
    pub original_size: u64,
    #[serde(default)]
    pub compressed_size: u64,
    #[serde(default)]
    pub deduplicated_size: u64,
    #[serde(default)]
    pub nfiles: u64,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub finished: bool,
}

impl fmt::Display for ProgressArchive {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            gettextf(
                "Backed up data: {}",
                &[&glib::format_size(self.original_size)]
            )
        )
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct ProgressPercent {
    #[serde(default)]
    msgid: Operation,
    finished: bool,
    #[serde(default)]
    message: String,
    current: Option<u64>,
    total: Option<u64>,
}

impl fmt::Display for ProgressPercent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let (Some(current), Some(total)) = (self.current, self.total) {
            let current = if current > 0 { current - 1 } else { current };

            let percent = current as f64 / total as f64 * 100.0;
            write!(
                f,
                "{} - {}",
                self.msgid,
                gettextf(
                    // xgettext:no-c-format
                    "Operation {} % completed ({}/{})",
                    &[
                        &format!("{percent:.0}"),
                        &current.to_string(),
                        &total.to_string()
                    ]
                )
            )
        } else if self.finished {
            write!(f, "{} - {}", self.msgid, gettext("Operation completed."))
        } else if !self.message.is_empty() {
            write!(f, "{} - {}", self.msgid, self.message)
        } else {
            write!(f, "{}", self.msgid)
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct ProgressMessage {
    #[serde(default)]
    msgid: Operation,
    #[serde(default)]
    message: String,
}

impl fmt::Display for ProgressMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.message.is_empty() {
            write!(f, "{}", self.msgid)
        } else {
            write!(f, "{} - {}", self.msgid, self.message)
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    #[serde(rename = "cache.begin_transaction")]
    CacheBeginTransaction,
    #[serde(rename = "cache.download_chunks")]
    CacheDownloadChunks,
    #[serde(rename = "cache.commit")]
    CacheCommit,
    #[serde(rename = "cache.sync")]
    CacheSync,
    #[serde(rename = "repository.compact_segments")]
    RepositoryCompactSegments,
    #[serde(rename = "repository.replay_segments")]
    RepositoryReplaySegments,
    #[serde(rename = "repository.check")]
    RepositoryCheck,
    #[serde(rename = "check.verify_data")]
    CheckVerifyData,
    #[serde(rename = "check.rebuild_manifest")]
    CheckRebuildManifest,
    Extract,
    #[serde(rename = "extract.permissions")]
    ExtractPermissions,
    #[serde(rename = "archive.delete")]
    ArchiveDelete,
    #[serde(rename = "archive.calc_stats")]
    ArchiveCalcStats,
    Prune,
    #[serde(rename = "upgrade.convert_segments")]
    UpgradeConvertSegments,
    Unspecified,
    #[serde(other)]
    Unknown,
}

impl Default for Operation {
    fn default() -> Self {
        Self::Unspecified
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match self {
            Self::CacheBeginTransaction => gettext("Beginning cache transaction"),
            Self::CacheDownloadChunks => gettext("Downloading cache data"),
            Self::CacheCommit => gettext("Writing cache"),
            Self::CacheSync => gettext("Synchronizing cache"),
            Self::RepositoryCompactSegments => gettext("Compacting repository"),
            Self::RepositoryReplaySegments => gettext("Updating repository"),
            Self::RepositoryCheck => gettext("Checking repository"),
            Self::CheckVerifyData => gettext("Verifying data"),
            Self::CheckRebuildManifest => gettext("Rebuilding main database"),
            Self::Extract => gettext("Extracting data"),
            Self::ExtractPermissions => gettext("Extracting permissions"),
            Self::ArchiveDelete => gettext("Marking archives as deleted"),
            Self::ArchiveCalcStats => gettext("Calculating archive statistics"),
            Self::Prune => gettext("Marking old archives as deleted"),
            Self::UpgradeConvertSegments => gettext("Upgrading repository"),
            Self::Unspecified => gettext("Unspecified operation"),
            Self::Unknown => gettext("Unknown operation"),
        };

        write!(f, "{msg}")
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct QuestionPrompt {
    #[serde(default)]
    msgid: QuestionId,
    #[serde(default)]
    message: String,
}

impl fmt::Display for QuestionPrompt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", gettext("Handling a Question"))
    }
}

impl QuestionPrompt {
    pub fn question_prompt(&self) -> String {
        let msg = match self.msgid {
            QuestionId::UnknownUnencryptedRepoAccessIsOk => {
                gettext("Attempting to access a previously unknown unencrypted repository.")
            }
            QuestionId::RelocatedRepoAccessIsOk => {
                let pattern: regex::Regex = regex::Regex::new(
                    r".*at location (\S+) .*previously located at (\S+).*",
                )
                .expect("Regex to be valid");

                let locations = if let Some(captures) = pattern.captures(&self.message).ok().flatten() {
                    (captures.get(1), captures.get(2))
                } else {
                    (None, None)
                };

                if let (Some(current), Some(previous)) = locations {
                    gettextf(
                        "The backup repository at location “{}” was previously located at “{}”.",
                        &[current.as_str(), previous.as_str()],
                    )
                } else {
                    gettext("The backup repository was previously located at a different location.")
                }
            }
            QuestionId::CheckIKnowWhatIAmDoing => gettext("This is a potentially dangerous function. Repairing a repository might lead to data loss (for kinds of corruption it is not capable of dealing with). BE VERY CAREFUL!"),
            QuestionId::DeleteIKnowWhatIAmDoing => gettext("You requested to delete the repository completely, including all backup archives it contains."),
            QuestionId::Unknown => gettextf("Unexpected question from borgbackup: “{}”", &[&self.message]),
        };

        // Translators: Combines statement from above and this question
        gettextf("{}\n\nDo you want to continue?", &[&msg])
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QuestionId {
    #[serde(rename = "BORG_UNKNOWN_UNENCRYPTED_REPO_ACCESS_IS_OK")]
    UnknownUnencryptedRepoAccessIsOk,

    #[serde(rename = "BORG_RELOCATED_REPO_ACCESS_IS_OK")]
    RelocatedRepoAccessIsOk,

    #[serde(rename = "BORG_CHECK_I_KNOW_WHAT_I_AM_DOING")]
    CheckIKnowWhatIAmDoing,

    #[serde(rename = "BORG_DELETE_I_KNOW_WHAT_I_AM_DOING")]
    DeleteIKnowWhatIAmDoing,

    Unknown,
}

impl Default for QuestionId {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LogMessage {
    pub levelname: LogLevel,
    pub name: String,
    pub message: String,
    #[serde(default)]
    pub msgid: MsgId,
}

impl std::fmt::Display for LogMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.message.is_empty() {
            write!(f, "")
        } else if matches!(self.msgid, MsgId::Undefined) {
            write!(f, "{}: {}", self.levelname, self.message)
        } else {
            write!(f, "{}: {} – {}", self.levelname, self.msgid, self.message)
        }
    }
}

impl std::error::Error for LogMessage {}

#[derive(Clone, Debug)]
pub struct BorgUnparsableErr {
    pub stderr: String,
}

impl std::fmt::Display for BorgUnparsableErr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {}", gettext("Standard error output"), self.stderr)
    }
}

impl Default for MsgId {
    fn default() -> Self {
        Self::Undefined
    }
}

#[derive(Deserialize)]
pub struct MsgIdHelper {
    pub msgid: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum LogEntry {
    ParsedErr(LogMessage),
    UnparsableErr(String),
}

impl LogEntry {
    pub fn message(&self) -> String {
        match &self {
            Self::ParsedErr(LogMessage { ref message, .. }) => message.to_string(),
            Self::UnparsableErr(ref message) => message.to_string(),
        }
    }

    pub fn id(&self) -> Option<MsgId> {
        match self {
            Self::ParsedErr(message) => Some(message.msgid.clone()),
            Self::UnparsableErr(_) => None,
        }
    }

    pub fn level(&self) -> LogLevel {
        match self {
            Self::ParsedErr(message) => message.levelname.clone(),
            Self::UnparsableErr(_) => LogLevel::Undefined,
        }
    }

    pub fn has_borg_msgid(&self, msgid_needle: &MsgId) -> bool {
        if let Self::ParsedErr(x) = self {
            if x.msgid == *msgid_needle {
                return true;
            }
        }

        false
    }
}

pub type LogCollection = Vec<LogEntry>;

pub trait LogExt {
    fn max_log_level(&self) -> Option<LogLevel>;
    fn to_string(&self) -> String;
    fn filter_handled(self) -> Self;
    fn filter_hidden(self) -> Self;
}

impl LogExt for LogCollection {
    fn max_log_level(&self) -> Option<LogLevel> {
        self.iter().map(|e| e.level()).max()
    }

    fn to_string(&self) -> String {
        self.iter()
            .map(|m| format!("{}", &m))
            .collect::<Vec<String>>()
            .join("\n")
    }

    /// Not considered for completed with error/warning status
    fn filter_handled(self) -> Self {
        #[allow(clippy::nonminimal_bool)]
        self.into_iter()
            .filter(|x| {
                !matches!(x.id(), Some(MsgId::PassphraseWrong))
                    &&
                    // The hint of 'ConnectionClosedWithHint'
                    !(x.has_borg_msgid(&MsgId::Undefined) && x.message().starts_with("Remote: "))
            })
            .collect()
    }

    /// Connection errors are not filtered from output
    fn filter_hidden(self) -> Self {
        self.into_iter()
            .filter(|x| !matches!(x.id(), Some(MsgId::PassphraseWrong)))
            .collect()
    }
}

impl std::fmt::Display for LogEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::ParsedErr(e) => write!(f, "{e}"),
            Self::UnparsableErr(s) => write!(f, "Unknown Message: {s}"),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
    Undefined,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let text = match self {
            Self::Debug => gettext("Debug"),
            Self::Info => gettext("Info"),
            Self::Warning => gettext("Warning"),
            Self::Error => gettext("Error"),
            Self::Critical => gettext("Critical"),
            Self::Undefined => gettext("Undefined"),
        };
        write!(f, "{text}")
    }
}
