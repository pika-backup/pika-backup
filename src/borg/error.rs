use super::msg::*;
use gettextrs::gettext;

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
        Borg(err: LogCollection) {
            from()
            display("{}", err)
        }
        BorgCreate(err: CreateLogCollection) {
            from()
            display("{}", err)
        }
        BorgReturnCode(err: ReturnCodeError) { from() }
        PasswordMissing { from(secret_service::Error) }
        UserAborted { display("{}", gettext("Aborted through user input.")) }
        ThreadPanicked { display("{}", gettext("The operation terminated unexpectedly.")) }
        Other(err: String) { from() }
    }
}

impl Error {
    pub fn has_borg_msgid(&self, msgid_needle: &MsgId) -> bool {
        match self {
            Self::Borg(LogCollection { messages, .. })
            | Self::BorgCreate(CreateLogCollection { messages, .. }) => {
                for message in messages {
                    if message.has_borg_msgid(msgid_needle) {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }
}
