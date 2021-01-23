use super::msg::*;
use gettextrs::gettext;

#[derive(Debug)]
pub struct ReturnCodeErr {
    pub code: Option<i32>,
}

impl ReturnCodeErr {
    pub fn new(code: Option<i32>) -> Self {
        Self { code }
    }
}

impl std::fmt::Display for ReturnCodeErr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Return code err: {:?}", self.code)
    }
}

impl std::error::Error for ReturnCodeErr {}

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Io(err: std::io::Error) { from() }
        Json(err: serde_json::error::Error) { from () }
        Unix(err: nix::Error) { from() }
        Borg(err: LogMessageCollection) {
            from()
            display("{}", err)
        }
        BorgCode(err: ReturnCodeErr) { from() }
        PasswordMissing { from(secret_service::Error) }
        UserAborted { display("{}", gettext("Aborted through user input")) }
        ThreadPanicked { display("{}", gettext("The operation terminated unexpectedly.")) }
        Other(err: String) { from() }
    }
}

impl Error {
    pub fn has_borg_msgid(&self, msgid_needle: &MsgId) -> bool {
        match self {
            Self::Borg(LogMessageCollection { messages }) => {
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
