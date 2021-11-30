use gettextrs::gettext;

pub type MsgId = super::error::Failure;

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

    pub fn is_connection_error(&self) -> bool {
        matches!(
            self.id(),
            Some(MsgId::ConnectionClosed)
                | Some(MsgId::ConnectionClosedWithHint)
                | Some(MsgId::ConnectionClosedWithHint_(_))
        )
    }
}

pub type LogCollection = Vec<LogMessageEnum>;

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
        if matches!(self.msgid, MsgId::Undefined) {
            write!(f, "{}: {}", self.levelname, self.message)
        } else {
            write!(f, "{}: {} – {}", self.levelname, self.msgid, self.message)
        }
    }
}

impl std::error::Error for LogMessage {}

#[derive(Debug)]
pub struct BorgUnparsableErr {
    pub stderr: String,
}

impl std::fmt::Display for BorgUnparsableErr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {}", gettext("Standard error output"), self.stderr)
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
        write!(f, "{}", text)
    }
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
