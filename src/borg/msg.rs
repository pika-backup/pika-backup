use super::json;

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub enum MsgId {
    ConnectionClosed,
    ConnectionClosedWithHint,
    LockTimeout,
    PassphraseWrong,
    #[serde(rename = "Repository.DoesNotExist")]
    RepositoryDoesNotExist,
    Other(String),
    #[serde(other)]
    Undefined,
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

#[derive(Clone, Debug)]
pub enum LogMessageEnum {
    ParsedErr(LogMessage),
    UnparsableErr(String),
}

impl LogMessageEnum {
    pub fn message(&self) -> String {
        match &self {
            Self::ParsedErr(LogMessage { ref message, .. }) => message.to_string(),
            Self::UnparsableErr(ref message) => message.to_string(),
        }
    }

    pub fn level(&self) -> LogLevel {
        match self {
            Self::ParsedErr(message) => message.levelname.clone(),
            Self::UnparsableErr(_) => LogLevel::None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LogCollection {
    pub messages: Vec<LogMessageEnum>,
    pub level: LogLevel,
}

impl LogCollection {
    pub fn new(messages: Vec<LogMessageEnum>) -> Self {
        let level = messages
            .iter()
            .map(|e| e.level())
            .max()
            .unwrap_or(LogLevel::None);

        Self { messages, level }
    }
}

#[derive(Clone, Debug)]
pub struct CreateLogCollection {
    pub messages: Vec<LogMessageEnum>,
    pub level: LogLevel,
    pub stats: Option<json::Stats>,
}

impl CreateLogCollection {
    pub fn new(messages: Vec<LogMessageEnum>, stats: Option<json::Stats>) -> Self {
        let mut collection = LogCollection::new(messages);

        // consider backups if 0 files as failed
        if let Some(json::Stats {
            archive:
                json::NewArchive {
                    stats: json::NewArchiveSize { nfiles, .. },
                    ..
                },
        }) = stats
        {
            if nfiles < 1 && collection.level < LogLevel::Error {
                collection.level = LogLevel::Error;
            }
        }

        Self {
            messages: collection.messages,
            level: collection.level,
            stats,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogMessage {
    pub levelname: LogLevel,
    pub name: String,
    pub message: String,
    #[serde(default)]
    pub msgid: MsgId,
}

impl std::fmt::Display for LogMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for LogMessage {}

#[derive(Debug)]
pub struct BorgUnparsableErr {
    pub stderr: String,
}

impl std::fmt::Display for BorgUnparsableErr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "STDERR({})", self.stderr)
    }
}

impl std::fmt::Display for LogMessageEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::ParsedErr(e) => write!(f, "{}", e),
            Self::UnparsableErr(s) => write!(f, "{}", s),
        }
    }
}

impl std::fmt::Display for LogCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.messages
                .iter()
                .map(|m| format!("{}", &m))
                .collect::<Vec<String>>()
                .join("\n")
        )
    }
}

impl std::fmt::Display for CreateLogCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.messages
                .iter()
                .map(|m| format!("{}", &m))
                .collect::<Vec<String>>()
                .join("\n")
        )
    }
}

impl LogMessageEnum {
    pub fn has_borg_msgid(&self, msgid_needle: &MsgId) -> bool {
        if let Self::ParsedErr(x) = self {
            if x.msgid == *msgid_needle {
                return true;
            }
        }

        false
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
    None,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum Progress {
    #[serde(rename = "archive_progress")]
    Archive(ProgressArchive),
    #[serde(rename = "progress_message")]
    Message {
        operation: u64,
        msgid: Option<String>,
        finished: bool,
        message: Option<String>,
    },
    #[serde(rename = "progress_percent")]
    Percent {
        operation: u64,
        msgid: Option<String>,
        finished: bool,
        message: Option<String>,
        current: Option<u64>,
        total: Option<u64>,
    },
}

#[derive(Deserialize, Clone, Debug)]
pub struct ProgressArchive {
    pub original_size: u64,
    pub compressed_size: u64,
    pub deduplicated_size: u64,
    pub nfiles: u64,
    pub path: String,
}
