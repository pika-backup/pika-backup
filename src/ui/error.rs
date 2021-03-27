use gtk::prelude::*;

use super::prelude::*;
use crate::borg;
use crate::config;
use crate::ui;

pub type Result<T> = std::result::Result<T, Error>;
pub type CombinedResult<T> = std::result::Result<T, Combined>;

quick_error! {
    #[derive(Debug)]
    pub enum Combined {
        Ui(err: Error) { from() }
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
            text: format!("{}", text),
            secondary_text: Some(format!("{}", secondary_text)),
        }
    }

    pub fn short<T: std::fmt::Display>(text: T) -> Self {
        Self {
            text: format!("{}", text),
            secondary_text: None,
        }
    }

    pub fn show(&self) {
        self.show_transient_for(&main_ui().window());
    }

    pub fn show_transient_for<W: IsA<gtk::Window> + IsA<gtk::Widget>>(&self, window: &W) {
        if let Some(secondary) = &self.secondary_text {
            ui::utils::show_error_transient_for(&self.text, secondary, window);
        } else {
            ui::utils::show_error_transient_for(&self.text, "", window);
        }
    }
}

#[derive(Debug)]
pub struct UserCanceled {}

impl UserCanceled {
    pub fn new() -> Self {
        UserCanceled {}
    }
}

quick_error! {
    #[derive(Debug, Eq, PartialEq)]
    pub enum Error {
        Message(err: Message) {
            from()
            from(err: futures::channel::oneshot::Canceled) ->
                (Message::short(gettext("The operation terminated unexpectedly.")))
            from(err: config::error::BackupExists) ->
                (Message::short(gettextf(
                    "Backup with id “{}” already exists.",
                    &[err.id.as_str()],
                )))
            from(err: config::error::BackupNotFound) ->
                (Message::short(gettextf(
                    "Could not find backup configuration with id “{}”",
                    &[err.id.as_str()],
                )))
        }
        UserCanceled { from (UserCanceled) }
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
    auto_visibility: Option<W>,
}

impl Handler<libhandy::ApplicationWindow> {
    pub fn run<F: std::future::Future<Output = Result<()>> + 'static>(f: F) {
        Self::new().error_transient_for(main_ui().window()).spawn(f);
    }
}

impl<W: IsA<gtk::Window> + IsA<gtk::Widget>> Handler<W> {
    pub fn new() -> Self {
        Self {
            transient_for: None,
            auto_visibility: None,
        }
    }

    pub fn error_transient_for(mut self, window: W) -> Self {
        self.transient_for = Some(window);
        self
    }

    pub fn dialog_auto_visibility(mut self, window: W) -> Self {
        self.auto_visibility = Some(window);
        self
    }

    pub fn spawn<F: std::future::Future<Output = Result<()>> + 'static>(&self, f: F) {
        let transient_for = self.transient_for.clone();
        let auto_visibility = self.auto_visibility.clone();

        glib::MainContext::default().spawn_local(async move {
            match f.await {
                Err(Error::Message(err)) => {
                    if let Some(auto_visibility) = auto_visibility {
                        auto_visibility.show();
                        ui::page_pending::back();
                    }

                    if let Some(transient_for) = transient_for {
                        err.show_transient_for(&transient_for);
                    } else {
                        err.show();
                    }
                }
                Err(Error::UserCanceled) => {
                    if let Some(auto_visibility) = auto_visibility {
                        ui::page_pending::back();
                        auto_visibility.show();
                    }
                }
                Ok(()) => {
                    if let Some(auto_visibility) = auto_visibility {
                        auto_visibility.close();
                    }
                }
            }
        });
    }
}
