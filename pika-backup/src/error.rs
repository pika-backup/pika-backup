use adw::prelude::*;
use common::{borg, config};

use super::prelude::*;
use super::widget::AppWindow;

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

#[derive(Debug)]
pub enum Combined {
    Ui(Error),
    Borg(borg::Error),
}

impl From<Error> for Combined {
    fn from(value: Error) -> Self {
        Self::Ui(value)
    }
}

impl From<borg::error::Error> for Combined {
    fn from(value: borg::error::Error) -> Self {
        Self::Borg(value)
    }
}

impl std::fmt::Display for Combined {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Combined::Ui(error) => error.fmt(f),
            Combined::Borg(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for Combined {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Combined::Ui(err) => Some(err),
            Combined::Borg(err) => Some(err),
        }
    }
}

#[derive(Debug)]
pub struct Message {
    text: String,
    secondary_text: Option<String>,
    notification_id: Option<String>,
    borg_error: Option<borg::Error>,
    markup: bool,
}

impl Message {
    pub fn new(text: impl std::fmt::Display, secondary_text: impl std::fmt::Display) -> Self {
        Self {
            text: text.to_string(),
            secondary_text: Some(secondary_text.to_string()),
            notification_id: None,
            borg_error: None,
            markup: false,
        }
    }

    pub fn with_borg(
        text: impl std::fmt::Display,
        secondary_text: impl std::fmt::Display,
        borg: borg::Error,
    ) -> Self {
        Self {
            text: text.to_string(),
            secondary_text: Some(secondary_text.to_string()),
            notification_id: None,
            borg_error: Some(borg),
            markup: false,
        }
    }

    pub fn with_notification_id(
        text: impl std::fmt::Display,
        secondary_text: impl std::fmt::Display,
        notification_id: impl std::fmt::Display,
    ) -> Self {
        Self {
            text: format!("{text}"),
            secondary_text: Some(format!("{secondary_text}")),
            notification_id: Some(notification_id.to_string()),
            borg_error: None,
            markup: false,
        }
    }

    pub fn short(text: impl std::fmt::Display) -> Self {
        Self {
            text: text.to_string(),
            secondary_text: None,
            notification_id: None,
            borg_error: None,
            markup: false,
        }
    }

    pub fn set_markup(&mut self, markup: bool) {
        self.markup = markup;
    }

    pub fn borg_error(&self) -> Option<&borg::Error> {
        self.borg_error.as_ref()
    }

    pub async fn show(&self) {
        self.show_transient_for(&main_ui().window()).await;
    }

    pub async fn show_transient_for(&self, widget: &impl IsA<gtk::Widget>) {
        fn ellipsize_multiline<S: std::fmt::Display>(x: S) -> String {
            let s = x.to_string();
            let vec = s.chars().collect::<Vec<_>>();

            if vec.len() > 510 {
                format!(
                    "{}\n…\n{}",
                    vec.iter().take(300).collect::<String>(),
                    vec.iter().rev().take(200).rev().collect::<String>()
                )
            } else {
                s
            }
        }

        let detail = self.secondary_text.as_deref().unwrap_or("");

        let primary_text = ellipsize_multiline(&self.text);
        let secondary_text = ellipsize_multiline(detail);
        tracing::warn!(
            "Displaying error:\n  {}\n  {}",
            &primary_text,
            &secondary_text
        );

        let window = crate::App::default().main_window();

        // Only display as dialog if focus and visible
        if window.is_mapped()
            && gtk::Window::list_toplevels().into_iter().any(|x| {
                x.downcast::<gtk::Window>()
                    .map(|w| w.is_active())
                    .unwrap_or_default()
            })
        {
            let dialog = adw::AlertDialog::builder()
                .heading(&primary_text)
                .body(&secondary_text)
                .prefer_wide_layout(true)
                .body_use_markup(self.markup)
                .build();

            dialog.add_responses(&[("close", &gettext("Close"))]);
            dialog.choose_future(Some(widget)).await;
        } else {
            let (title, mut body) = if secondary_text.is_empty() {
                (gettext("Pika Backup"), primary_text)
            } else {
                (primary_text, secondary_text)
            };

            if self.markup {
                // Remove markup for notification body
                body = gtk::pango::parse_markup(&body, '\0')
                    .map(|x| x.1.to_string())
                    .unwrap_or(body);
            }

            let notification = gio::Notification::new(&title);
            notification.set_body(Some(&body));

            adw_app().send_notification(self.notification_id.as_deref(), &notification);
        }
    }

    pub fn from_secret_service<T: std::fmt::Display>(text: T, err: oo7::Error) -> Self {
        if let oo7::Error::File(oo7::file::Error::Portal(portal_err)) = &err {
            let mut msg = gettext(
                "The keyring is not available. Pika Backup requires a keyring daemon (“secret service”) to store passwords. For installation instructions see the operating system documentation.",
            );
            msg.push_str(&portal_err.to_string());
            Self::new(text, msg)
        } else {
            Self::new(text, err)
        }
    }
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(secondary_text) = &self.secondary_text {
            write!(f, "{}\n{}", self.text, secondary_text)
        } else {
            write!(f, "{}", self.text)
        }
    }
}

impl std::error::Error for Message {}

#[derive(Debug)]
pub enum Error {
    Message(Box<Message>),
    UserCanceled,
}

impl Error {
    pub fn message(msg: Message) -> Self {
        Self::Message(Box::new(msg))
    }
}

impl From<config::error::BackupExists> for Error {
    fn from(value: config::error::BackupExists) -> Self {
        Self::message(Message::short(gettextf(
            "Backup with id “{}” already exists.",
            [value.id.as_str()],
        )))
    }
}

impl From<config::error::BackupNotFound> for Error {
    fn from(value: config::error::BackupNotFound) -> Self {
        Self::message(Message::short(gettextf(
            "Could not find backup configuration with id “{}”.",
            [value.id.as_str()],
        )))
    }
}

impl From<Message> for Error {
    fn from(value: Message) -> Self {
        Self::message(value)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Message(msg) => msg.fmt(f),
            Error::UserCanceled => write!(f, "{}", gettext("Canceled")), /* This should generally not appear anywhere, */
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Message(err) => Some(err),
            Error::UserCanceled => None,
        }
    }
}

impl Error {
    pub async fn show(&self) {
        if let Self::Message(err) = self {
            err.show().await;
        }
    }

    pub async fn show_transient_for(&self, window: &impl IsA<gtk::Widget>) {
        if let Self::Message(err) = self {
            err.show_transient_for(window).await;
        }
    }

    pub fn message_text(&self) -> &str {
        match self {
            Error::Message(msg) => &msg.text,
            Error::UserCanceled => "",
        }
    }

    pub fn message_secondary_text(&self) -> Option<&str> {
        match self {
            Error::Message(msg) => msg.secondary_text.as_deref(),
            Error::UserCanceled => None,
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
            Combined::Borg(borg::Error::Aborted(borg::Abort::User)) => Error::UserCanceled,
            Combined::Borg(err) => Message::with_borg(text, err.to_string(), err).into(),
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

pub struct Handler<W: IsA<gtk::Widget>> {
    transient_for: Option<W>,
}

impl Handler<AppWindow> {
    pub fn run<F: std::future::Future<Output = Result<()>> + 'static>(f: F) {
        Self::new().error_transient_for(main_ui().window()).spawn(f);
    }

    pub fn handle(result: Result<()>) {
        Self::new()
            .error_transient_for(main_ui().window())
            .spawn(async { result });
    }
}

impl<W: IsA<gtk::Widget>> Handler<W> {
    pub fn new() -> Self {
        Self {
            transient_for: None,
        }
    }

    pub fn error_transient_for(mut self, widget: W) -> Self {
        self.transient_for = Some(widget);
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
}

pub trait HandleError<T> {
    async fn handle_transient_for(self, window: &impl IsA<gtk::Widget>) -> Option<T>;
}

impl<T> HandleError<T> for Result<T> {
    async fn handle_transient_for(self, widget: &impl IsA<gtk::Widget>) -> Option<T> {
        match self {
            Ok(res) => Some(res),
            Err(Error::Message(err)) => {
                err.show_transient_for(widget).await;
                None
            }
            Err(Error::UserCanceled) => None,
        }
    }
}
