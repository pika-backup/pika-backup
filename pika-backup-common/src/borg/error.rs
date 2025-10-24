use quick_error::quick_error;
use serde::{Deserialize, Serialize};

use super::json;
use super::log_json::*;
use super::prelude::*;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct ReturnCodeError {
    pub code: Option<i32>,
}

impl ReturnCodeError {
    pub const fn new(code: Option<i32>) -> Self {
        Self { code }
    }
}

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Io(err: std::io::Error) { from() }
        Json(err: serde_json::error::Error) { from () }
        Unix(err: nix::Error) { from() }
        BorgReturnCode(err: ReturnCodeError) { from() }
        PasswordMissing { keyring_error: Option<String> }
        PasswordStorage(err: oo7::Error) {
            from()
            display("{}", gettext("Retrieving encryption password from the keyring failed. Pika Backup requires a keyring daemon (“secret service”) to store passwords. For installation instructions see the operating system documentation."))
        }
        ThreadPanicked { display("{}", gettext("The operation terminated unexpectedly.")) }
        ImplausiblePrune { display("{}", gettext("This delete operation would delete too many archives.")) }
        EmptyInclude { display("{}", gettext("No files selected to be included into backup.")) }
        Failed(err: Failure) {
            from()
            from(err: String) -> (Failure::Other(err))
            display("{}", err)
        }
        ChannelSend(err: smol::channel::SendError<super::Update>) { from() }
        Aborted(err: Abort) {
            from()
            display("{}", err)
        }
    }
}

impl std::convert::TryFrom<LogCollection> for Error {
    type Error = ();
    fn try_from(value: LogCollection) -> std::result::Result<Self, Self::Error> {
        let mut errors = value.iter().filter(|e| e.level() >= LogLevel::Error);

        let first_with_id = errors.clone().find(|e| e.id().is_some());

        if let Some(failure) = first_with_id.and_then(|e| e.id()) {
            match failure {
                // Find hint for ClosedWithHint
                Failure::ConnectionClosedWithHint => {
                    let hint = value
                        .iter()
                        .filter(|x| x.level() == LogLevel::Warning)
                        .next_back()
                        .map(|x| x.message());

                    if let Some(hint) = hint {
                        Ok(Failure::ConnectionClosedWithHint_(hint).into())
                    } else {
                        Ok(failure.into())
                    }
                }
                Failure::Exception => {
                    let hint = value
                        .iter()
                        .filter(|x| match x.level() {
                            // SSH error: Broken pipe
                            LogLevel::Error => x.message().contains("[Errno 32] Broken pipe"),
                            // Will be thrown by borg when the network is disabled manually / the
                            // wifi disconnects
                            LogLevel::Warning => x
                                .message()
                                .contains("Remote: Timeout, server borg not responding."),
                            _ => false,
                        })
                        .next_back()
                        .map(|_| gettext("Timeout"));

                    if let Some(hint) = hint {
                        Ok(Failure::ConnectionClosedWithHint_(hint).into())
                    } else {
                        Ok(format!(
                            "{}: {}",
                            failure,
                            first_with_id.map(|x| x.message()).unwrap_or_default()
                        )
                        .into())
                    }
                }
                Failure::Other(msgid) => Ok(format!(
                    "{}: {}",
                    msgid,
                    first_with_id.map(|x| x.message()).unwrap_or_default()
                )
                .into()),
                _ => Ok(failure.into()),
            }
        } else {
            errors.next().map(|x| x.message().into()).ok_or(())
        }
    }
}

/// The outcome of the backup operation
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Outcome {
    /// Backup has completed successfully
    Completed { stats: json::Stats },
    /// Backup was not started / was aborted due to external factors
    Aborted(Abort),
    /// The borg process has thrown an error that caused the backup to fail
    Failed(Failure),
}

impl Outcome {
    pub const fn is_completed(&self) -> bool {
        matches!(self, Outcome::Completed { .. })
    }
}

impl std::fmt::Display for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Completed { .. } => write!(f, "{}", gettext("Completed")),
            Self::Aborted(x) => write!(f, "{x}"),
            Self::Failed(x) => write!(f, "{x}"),
        }
    }
}

/// The backup was not started / was aborted due to external factors
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Abort {
    User,
    /// Too long on metered connection
    MeteredConnection,
    /// Too long on battery
    OnBattery,
    /// program was shutdown via signal
    Shutdown,
    /// program probably crashed while running
    LeftRunning,
    /// shell script configured by the user failed to run
    UserShellCommand(String),
    /// Unable to mount / access the repository during setup.
    /// Detailed error message in parameter.
    RepositoryNotAvailable(String),
    QuestionDuringSchedule(QuestionPrompt),
}

impl std::fmt::Display for Abort {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "{}", gettext("Aborted on user request.")),
            Self::MeteredConnection => write!(
                f,
                "{}",
                gettext("Aborted because only metered connection was available.")
            ),
            Self::OnBattery => write!(
                f,
                "{}",
                // Translators: Running backup was aborted because computer running on battery
                gettext("Aborted because too long not connected to power.")
            ),
            Self::Shutdown => write!(f, "{}", gettext("Aborted by system.")),
            Self::LeftRunning => write!(
                f,
                "{}",
                gettext("The program or system seems to have crashed.")
            ),
            Self::UserShellCommand(msg) => {
                write!(f, "{}", gettextf("{}", [msg]))
            }
            Self::RepositoryNotAvailable(msg) => {
                write!(
                    f,
                    "{}",
                    gettextf("Unable to access backup repository: {}", [msg])
                )
            }
            Abort::QuestionDuringSchedule(question) => f.write_str(&question.question_prompt()),
        }
    }
}

/// The borg process has thrown an error that caused the backup to fail
///
/// The borg message ids are annotated with the return codes just to keep them
/// in the same order as the borg docs to make it easier to check if we are
/// missing ids.
///
/// <https://borgbackup.readthedocs.io/en/stable/internals/frontends.html#message-ids>
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Failure {
    // # Borg message IDs
    /// RC 10
    #[serde(rename = "Repository.AlreadyExists")]
    RepositoryAlreadyExists,
    /// RC 12
    #[serde(rename = "Repository.CheckNeeded")]
    RepositoryCheckNeeded,
    /// RC 13
    #[serde(rename = "Repository.DoesNotExist")]
    RepositoryDoesNotExist,
    /// RC 14
    #[serde(rename = "Repository.InsufficientFreeSpaceError")]
    RepositoryInsufficientFreeSpaceError,
    // RC 15
    #[serde(rename = "Repository.InvalidRepository")]
    RepositoryInvalidRepository,
    // RC 16
    #[serde(rename = "Repository.InvalidRepositoryConfig")]
    RepositoryInvalidRepositoryConfig,
    // RC 18
    #[serde(rename = "Repository.ParentPathDoesNotExist")]
    RepositoryParentPathDoesNotExist,
    // RC 19
    #[serde(rename = "Repository.PathAlreadyExists")]
    RepositoryPathAlreadyExists,
    // RC 20
    #[serde(rename = "Repository.StorageQuotaExceeded")]
    RepositoryStorageQuotaExceeded,
    // RC 21
    #[serde(rename = "Repository.PathPermissionDenied")]
    RepositoryPathPermissionDenied,

    // RC 25
    MandatoryFeatureUnsupported,
    // RC 27
    UnsupportedManifestError,

    // RC 30
    #[serde(rename = "Archive.AlreadyExists")]
    ArchiveAlreadyExists,
    // RC 31
    #[serde(rename = "Archive.DoesNotExist")]
    ArchiveDoesNotExist,

    /// RC 52
    PassphraseWrong,

    /// RC 62
    #[serde(rename = "Cache.RepositoryAccessAborted")]
    CacheRepositoryAccessAborted,

    /// RC 72
    LockFailed,
    /// RC 73
    LockTimeout,

    /// RC 80
    ConnectionClosed,
    /// RC 81
    ConnectionClosedWithHint,

    /// Connection closed with manually added hint
    ConnectionClosedWithHint_(String),

    // # General
    /// Unknown borg exception
    Exception,
    /// Other (one-off) exception
    Other(String),

    /// Fallback
    #[serde(other)]
    Undefined,
}

impl Failure {
    pub const fn is_connection_error(&self) -> bool {
        matches!(
            self,
            Self::ConnectionClosed
                | Self::ConnectionClosedWithHint
                | Self::ConnectionClosedWithHint_(_)
        )
    }
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let text = match self {
            // RC 10+
            Self::RepositoryAlreadyExists => {
                gettext("A repository already exists at this location.")
            }
            Self::RepositoryCheckNeeded => gettext(
                "Inconsistencies were detected in the repository. Running a data integrity check is needed.",
            ),
            Self::RepositoryDoesNotExist => gettext("No repository exists at this location."),
            Self::RepositoryInsufficientFreeSpaceError => {
                gettext("Not enough free space in repository.")
            }
            Self::RepositoryInvalidRepository => {
                gettext("The configured location does not contain a valid repository.")
            }
            Self::RepositoryInvalidRepositoryConfig => gettext(
                "The configured location does not contain a valid repository configuration.",
            ),
            Self::RepositoryParentPathDoesNotExist => {
                gettext("The configured location does not exist.")
            }
            Self::RepositoryPathAlreadyExists => {
                gettext("The selected location does already exist.")
            }
            Self::RepositoryStorageQuotaExceeded => gettext(
                "The repositories storage quota has been reached. Try deleting older archives that are no longer needed.",
            ),
            Self::RepositoryPathPermissionDenied => gettext(
                "Permission to access the configured location has been denied. Check the file permissions.",
            ),

            // RC 25+
            Self::MandatoryFeatureUnsupported | Self::UnsupportedManifestError => gettext(
                "A newer version of Pika Backup is required to access this repository. The repository uses unsupported features.",
            ),

            // RC 30+
            Self::ArchiveAlreadyExists => {
                gettext("The selected location already contains a repository.")
            }
            Self::ArchiveDoesNotExist => {
                gettext("The configured location does not contain an archive.")
            }

            // RC 50+
            Self::PassphraseWrong => gettext("Invalid encryption password."),

            // RC 60+
            Self::CacheRepositoryAccessAborted => gettext("Repository access was aborted"),

            // RC 70+
            Self::LockFailed => gettext("Failed to lock repository."),
            Self::LockTimeout => gettext("Repository already in use."),

            // RC 80+
            Self::ConnectionClosed | Self::ConnectionClosedWithHint => {
                gettext("Connection closed by remote host.")
            }

            // Manually added data
            Self::ConnectionClosedWithHint_(hint) => {
                gettextf("Connection closed by remote host: “{}”", [hint])
            }

            // General
            Self::Exception => gettext("Exception"),
            Self::Other(string) => string.to_string(),
            Self::Undefined => gettext("Unspecified error."),
        };

        write!(f, "{}", &text)
    }
}
