use gtk::prelude::*;

use super::prelude::*;
use crate::borg;
use crate::config;
use crate::ui;

pub type Result<T> = std::result::Result<T, Error>;
pub type CombinedResult<T> = std::result::Result<T, Combined>;

pub trait CombinedResultExt<T> {
    fn is_borg_err_user_aborted(&self) -> bool;
}

impl<T> CombinedResultExt<T> for CombinedResult<T> {
    fn is_borg_err_user_aborted(&self) -> bool {
        matches!(
            self,
            Err(Combined::Borg(borg::Error::Aborted(
                borg::error::Abort::User
            )))
        )
    }
}

quick_error! {
    #[derive(Debug)]
    pub enum Combined {
        Ui(err: Error) {
            from()
            from(_err: UserCanceled) -> (Error::UserCanceled)
        }
        Borg(err: borg::Error) { from () }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Message {
    text: String,
    secondary_text: Option<String>,
}

impl Message {
    pub fn new<T: std::fmt::Display, S: std::fmt::Display>(text: T, secondary_text: S) -> Self {
        Self {
            text: format!("{text}"),
            secondary_text: Some(format!("{secondary_text}")),
        }
    }

    pub fn short<T: std::fmt::Display>(text: T) -> Self {
        Self {
            text: format!("{text}"),
            secondary_text: None,
        }
    }

    pub async fn show(&self) {
        self.show_transient_for(&main_ui().window()).await;
    }

    pub async fn show_transient_for<W: IsA<gtk::Window> + IsA<gtk::Widget>>(&self, window: &W) {
        if let Some(secondary) = &self.secondary_text {
            ui::utils::show_error_transient_for(&self.text, secondary, window).await;
        } else {
            ui::utils::show_error_transient_for(&self.text, "", window).await;
        }
    }

    pub fn from_secret_service<T: std::fmt::Display>(text: T, err: oo7::Error) -> Self {
        if matches!(
            err,
            oo7::Error::Portal(oo7::portal::Error::CancelledPortalRequest)
        ) {
            Self::new(text, gettext("The keyring is not available. Pika Backup requires a keyring daemon (“secret service”) to store passwords. For installation instructions see the operating system documentation."))
        } else {
            Self::new(text, err)
        }
    }
}

#[derive(Debug)]
pub struct UserCanceled;

impl UserCanceled {
    pub const fn new() -> Self {
        Self
    }
}

quick_error! {
    #[derive(Debug, Eq, PartialEq)]
    pub enum Error {
        Message(err: Message) {
            from()
            from(err: config::error::BackupExists) ->
                (Message::short(gettextf(
                    "Backup with id “{}” already exists.",
                    &[err.id.as_str()],
                )))
            from(err: config::error::BackupNotFound) ->
                (Message::short(gettextf(
                    "Could not find backup configuration with id “{}”.",
                    &[err.id.as_str()],
                )))
        }
        UserCanceled { from (UserCanceled) }
    }
}

impl Error {
    pub async fn show(&self) {
        if let Self::Message(err) = self {
            err.show().await;
        }
    }
}

pub trait ErrorToMessage<R> {
    fn err_to_msg<T: std::fmt::Display>(self, text: T) -> Result<R>;
}

impl<R, E: std::fmt::Display> ErrorToMessage<R> for std::result::Result<R, E> {
    fn err_to_msg<T: std::fmt::Display>(self, text: T) -> Result<R> {
        self.map_err(|err| Message::new(text, err).into())
    }
}

pub trait CombinedToError<R> {
    fn into_message<T: std::fmt::Display>(self, text: T) -> Result<R>;
    fn into_borg_error(self) -> Result<borg::Result<R>>;
}

impl<R> CombinedToError<R> for std::result::Result<R, Combined> {
    fn into_message<T: std::fmt::Display>(self, text: T) -> Result<R> {
        self.map_err(|err| match err {
            Combined::Ui(err) => err,
            Combined::Borg(err) => Message::new(text, err).into(),
        })
    }
    fn into_borg_error(self) -> Result<borg::Result<R>> {
        match self {
            Ok(r) => Ok(Ok(r)),
            Err(Combined::Borg(err)) => Ok(Err(err)),
            Err(Combined::Ui(err)) => Err(err),
        }
    }
}

#[derive(Default)]
pub struct Handler<W: IsA<gtk::Window> + IsA<gtk::Widget>> {
    transient_for: Option<W>,
}

impl Handler<adw::ApplicationWindow> {
    pub fn run<F: std::future::Future<Output = Result<()>> + 'static>(f: F) {
        Self::new().error_transient_for(main_ui().window()).spawn(f);
    }

    pub fn handle(result: Result<()>) {
        Self::new()
            .error_transient_for(main_ui().window())
            .spawn(async { result });
    }
}

impl<W: IsA<gtk::Window> + IsA<gtk::Widget>> Handler<W> {
    pub fn new() -> Self {
        Self {
            transient_for: None,
        }
    }

    pub fn error_transient_for(mut self, window: W) -> Self {
        self.transient_for = Some(window);
        self
    }

    pub fn spawn<F: std::future::Future<Output = Result<()>> + 'static>(&self, f: F) {
        let transient_for = self.transient_for.clone();

        glib::MainContext::default().spawn_local(async move {
            match f.await {
                Err(Error::Message(err)) => {
                    if let Some(transient_for) = transient_for {
                        err.show_transient_for(&transient_for).await;
                    } else {
                        err.show().await;
                    }
                }
                Err(Error::UserCanceled) | Ok(()) => {}
            }
        });
    }

    pub fn handle_sync(&self, result: Result<()>) {
        self.spawn(async { result });
    }
}
