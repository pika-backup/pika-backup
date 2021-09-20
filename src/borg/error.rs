use super::json;
use super::msg::*;
use super::prelude::*;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct ReturnCodeError {
    pub code: Option<i32>,
}

impl ReturnCodeError {
    pub fn new(code: Option<i32>) -> Self {
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
        PasswordMissing { from(secret_service::Error) }
        ThreadPanicked { display("{}", gettext("The operation terminated unexpectedly.")) }
        Failed(err: Failure) {
            from()
            from(err: String) -> (Failure::Other(err))
            display("{}", err)
        }
        Aborted(err: Abort) { from() }
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
                        .rev()
                        .next()
                        .map(|x| x.message());

                    if let Some(hint) = hint {
                        Ok(Failure::ConnectionClosedWithHint_(hint).into())
                    } else {
                        Ok(failure.into())
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Outcome {
    Completed { stats: json::Stats },
    Aborted(Abort),
    Failed(Failure),
}

impl std::fmt::Display for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Completed { .. } => write!(f, "{}", gettext("Completed")),
            Self::Aborted(x) => write!(f, "{}", x),
            Self::Failed(x) => write!(f, "{}", x),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Abort {
    User,
}

impl std::fmt::Display for Abort {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "{}", gettext("Aborted on user request.")),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Failure {
    // borg message ids
    ConnectionClosed,
    ConnectionClosedWithHint,
    LockTimeout,
    PassphraseWrong,
    #[serde(rename = "Repository.AlreadyExists")]
    RepositoryAlreadyExists,
    #[serde(rename = "Repository.DoesNotExist")]
    RepositoryDoesNotExist,
    // with manually added hint
    ConnectionClosedWithHint_(String),
    // general
    Other(String),
    #[serde(other)]
    Undefined,
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let text = match self {
            Self::ConnectionClosed | Self::ConnectionClosedWithHint => {
                gettext("Connection closed by remote host.")
            }
            Self::LockTimeout => gettext("Repository already in use."),
            Self::PassphraseWrong => gettext("Invalid encryption password."),
            Self::RepositoryAlreadyExists => {
                gettext("A repository already exists at this location.")
            }
            Self::RepositoryDoesNotExist => gettext("No repository exists at this location."),

            Self::ConnectionClosedWithHint_(hint) => {
                gettextf("Connection closed by remote host: “{}“", &[hint])
            }
            Self::Other(string) => string.to_string(),
            Self::Undefined => gettext("Unspecified error."),
        };

        write!(f, "{}", &text)
    }
}
