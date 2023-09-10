pub mod borg;
pub mod config_io;
pub mod df;
pub mod duration;
pub mod ext;
pub mod flatpak_info;
pub mod password_storage;
pub mod repo_cache;

use crate::ui::prelude::*;
use adw::prelude::*;

use crate::config;

use ashpd::desktop::background;
use std::convert::TryInto;
use std::fmt::Display;
use std::io::Read;
use std::os::unix::process::CommandExt;

#[derive(Clone, Copy, Debug, glib::ValueDelegate)]
#[value_delegate(from = u8)]
pub enum StatusLevel {
    Ok,
    Neutral,
    Warning,
    Error,
    Spinner,
}

impl Default for StatusLevel {
    fn default() -> Self {
        Self::Neutral
    }
}

impl From<u8> for StatusLevel {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Ok,
            1 => Self::Neutral,
            2 => Self::Warning,
            3 => Self::Error,
            4 => Self::Spinner,
            _ => Self::Neutral,
        }
    }
}

impl<'a> From<&'a StatusLevel> for u8 {
    fn from(v: &'a StatusLevel) -> Self {
        *v as u8
    }
}

impl From<StatusLevel> for u8 {
    fn from(v: StatusLevel) -> Self {
        v as u8
    }
}

/// Returns a relative path for sub directories of home
pub fn rel_path(path: &std::path::Path) -> std::path::PathBuf {
    if let Ok(rel_path) = path.strip_prefix(glib::home_dir().as_path()) {
        rel_path.to_path_buf()
    } else {
        path.to_path_buf()
    }
}

/// Checks if a directory is most likely a borg repository. Performed checks are
///
/// - `data/` exists and is a directory
/// - `config` exists and contains the string `[repository]`
pub fn is_backup_repo(path: &std::path::Path) -> bool {
    trace!("Checking path if it is a repo '{}'", &path.display());
    if let Ok(data) = std::fs::File::open(path.join("data")).and_then(|x| x.metadata()) {
        if data.is_dir() {
            if let Ok(mut cfg) = std::fs::File::open(path.join("config")) {
                if let Ok(metadata) = cfg.metadata() {
                    if metadata.len() < 1024 * 1024 {
                        let mut content = String::new();
                        let _result = cfg.read_to_string(&mut content);
                        if content.contains("[repository]") {
                            trace!("Is a repository");
                            return true;
                        }
                    }
                }
            }
        }
    };

    trace!("Is not a repository");
    false
}

pub fn cache_dir() -> std::path::PathBuf {
    [glib::user_cache_dir(), env!("CARGO_PKG_NAME").into()]
        .iter()
        .collect()
}

pub async fn background_permission() -> Result<()> {
    let generic_msg = gettext("Request to run in background failed");

    if !*crate::globals::APP_IS_SANDBOXED {
        // start
        crate::utils::dbus::fdo_proxy()
            .await
            .err_to_msg(&generic_msg)?
            .start_service_by_name(crate::DAEMON_APP_ID.try_into().unwrap(), Default::default())
            .await
            .err_to_msg(&generic_msg)?;

        // without flatpak we can always run in background
        Ok(())
    } else {
        let response = background::Background::request()
            .identifier(ashpd::WindowIdentifier::default())
            .reason(&*gettext("Schedule backups and continue running backups."))
            .auto_start(true)
            .command(std::iter::once(crate::DAEMON_BINARY))
            // Do not use dbus-activation because that would start the UI
            // See <https://gitlab.gnome.org/Teams/Design/hig-www/-/issues/107>
            .dbus_activatable(false)
            .send()
            .await
            .and_then(|request| request.response());

        let is_rejected = match &response {
            Ok(background) if !background.run_in_background() || !background.auto_start() => true,
            Err(ashpd::Error::Response(ashpd::desktop::ResponseError::Cancelled)) => true,
            _ => false,
        };

        if is_rejected {
            return Err(Message::new(gettext("Run in background disabled"),
            gettext("Scheduled backup functionality and continuing backups in the background will not be available. The “run in background” option can be enabled via the system settings.")).into());
        }

        match response {
            Err(err) => {
                warn!("Background portal response: {:?}", err);

                Err(
                    Message::new(
                        gettext(&generic_msg),
                        gettext("The system does not support running Pika Backup in the background. Scheduled backup functionality and continuing backups in the background will not be available. This is either a malfunction, misconfiguration or other issue with the xdg-desktop-portal. Please report this issue in your distribution issue tracker.")
                    + "\n\n" + &err.to_string()
                    ).into()
                )
            }
            Ok(_) => {
                let mut command = std::process::Command::new(crate::DAEMON_BINARY);

                if let Ok(debug) = std::env::var("G_MESSAGES_DEBUG") {
                    command.env("G_MESSAGES_DEBUG", debug);
                }

                unsafe {
                    command.pre_exec(|| {
                        nix::unistd::setsid()
                            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
                            .map(|_| ())
                    });
                }

                command
                    .spawn()
                    .err_to_msg(gettext("Failed to Start Monitor"))
                    .map(|_| ())?;

                fix_flatpak_autostart()
            }
        }
    }
}

pub trait LookupActiveConfigId {
    type Item;
    #[doc(alias = "get_active")]
    fn active(&self) -> Result<&Self::Item>;
    #[doc(alias = "get_active_mut")]
    fn active_mut(&mut self) -> Result<&mut Self::Item>;
}

#[allow(clippy::implicit_hasher)]
impl<T> LookupActiveConfigId for std::collections::BTreeMap<ConfigId, T> {
    type Item = T;

    fn active(&self) -> Result<&T> {
        Ok(self.get_result(&active_config_id_result()?)?)
    }

    fn active_mut(&mut self) -> Result<&mut T> {
        Ok(self.get_result_mut(&active_config_id_result()?)?)
    }
}

impl LookupActiveConfigId for config::Backups {
    type Item = config::Backup;

    fn active(&self) -> Result<&config::Backup> {
        Ok(self.get_result(&active_config_id_result()?)?)
    }

    fn active_mut(&mut self) -> Result<&mut config::Backup> {
        Ok(self.get_result_mut(&active_config_id_result()?)?)
    }
}

impl LookupActiveConfigId for config::Histories {
    type Item = config::history::History;

    fn active(&self) -> Result<&Self::Item> {
        self.0.active()
    }

    fn active_mut(&mut self) -> Result<&mut Self::Item> {
        self.0.active_mut()
    }
}

impl<C: LookupActiveConfigId> LookupActiveConfigId for config::Writeable<C> {
    type Item = C::Item;

    fn active(&self) -> Result<&Self::Item> {
        self.current_config.active()
    }

    fn active_mut(&mut self) -> Result<&mut Self::Item> {
        self.current_config.active_mut()
    }
}

pub trait SummarizeOperations {
    fn summarize_operations(&self) -> Option<String>;
}

impl SummarizeOperations
    for std::collections::BTreeMap<ConfigId, Rc<dyn super::operation::OperationExt>>
{
    fn summarize_operations(&self) -> Option<String> {
        match self.len() {
            0 => None,
            1 => self.first_key_value().map(|(_id, op)| op.name()),
            n => Some(ngettextf_(
                "One Backup Operation Running",
                "{} Backup Operations Running",
                n as u32,
            )),
        }
    }
}

fn active_config_id_result() -> Result<ConfigId> {
    ACTIVE_BACKUP_ID
        .get()
        .ok_or_else(|| Message::short("There is no active backup in the interface.").into())
}

pub async fn spawn_thread<P: core::fmt::Display, F, R>(name: P, task: F) -> Result<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let result = async_std::task::Builder::new()
        .name(name.to_string())
        .spawn(async { task() });

    Ok(result.err_to_msg(gettext("Failed to Create Thread"))?.await)
}

quick_error! {
    #[derive(Debug)]
    pub enum AsyncErr {
        ThreadPanicked { display("{}", gettext("The operation terminated unexpectedly.")) }
    }
}

pub fn folder_chooser<T: IsA<gtk::Window>>(title: &str, parent: &T) -> gtk::FileChooserNative {
    gtk::FileChooserNative::builder()
        .action(gtk::FileChooserAction::SelectFolder)
        .title(title)
        .accept_label(gettext("Select"))
        .modal(true)
        .transient_for(parent)
        .build()
}

pub async fn folder_chooser_dialog(title: &str) -> Option<gio::File> {
    let dialog = folder_chooser(title, &main_ui().window());

    if dialog.run_future().await == gtk::ResponseType::Accept {
        dialog.file()
    } else {
        None
    }
}

pub async fn paths(dialog: gtk::FileChooserNative) -> Result<Vec<std::path::PathBuf>> {
    dialog.run_future().await;

    let paths: Vec<_> = dialog
        .files()
        .snapshot()
        .into_iter()
        .filter_map(|obj| obj.downcast_ref::<gio::File>().and_then(|x| x.path()))
        .collect();

    if paths.is_empty() {
        Err(Error::UserCanceled)
    } else {
        Ok(paths)
    }
}

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

/// Ellipsizes a string at the end so that it is `max_len` characters long
pub fn ellipsize_end<S: std::fmt::Display>(x: S, max_len: usize) -> String {
    let mut text = x.to_string();

    if text.len() <= max_len {
        text
    } else {
        text.truncate(max_len.max(1) - 1);
        text.push('…');
        text
    }
}

pub fn show_notice<S: std::fmt::Display>(message: S) {
    warn!("Displaying notice:\n  {}", message);

    let toast = adw::Toast::builder()
        .title(message.to_string())
        .timeout(0)
        .build();

    main_ui().toast().add_toast(toast);

    if !crate::ui::app_window::is_displayed() {
        let notification = gio::Notification::new(&gettext("Pika Backup"));
        notification.set_body(Some(&message.to_string()));

        adw_app().send_notification(None, &notification);
    }
}

pub async fn show_borg_question(
    question: &crate::borg::log_json::QuestionPrompt,
) -> crate::borg::Response {
    let prompt = question.question_prompt();
    warn!("Displaying borg question: '{}'", prompt);

    // TODO: handle main window closed
    let response = ConfirmationDialog::new(
        &gettext("Warning"),
        &prompt,
        &gettext("Abort"),
        &gettext("Continue"),
    )
    .set_destructive(true)
    .ask()
    .await;

    match response {
        Ok(()) => crate::borg::Response::Yes,
        Err(_) => crate::borg::Response::No,
    }
}

pub async fn show_error_transient_for<
    S: std::fmt::Display,
    P: std::fmt::Display,
    W: IsA<gtk::Window>,
>(
    message: S,
    detail: P,
    window: &W,
) {
    let primary_text = ellipsize_multiline(message);
    let secondary_text = ellipsize_multiline(detail);
    warn!(
        "Displaying error:\n  {}\n  {}",
        &primary_text, &secondary_text
    );

    // Only display as dialog if focus and visible
    if crate::ui::app_window::is_displayed()
        && gtk::Window::list_toplevels().into_iter().any(|x| {
            x.downcast::<gtk::Window>()
                .map(|w| w.is_active())
                .unwrap_or_default()
        })
    {
        let dialog = adw::MessageDialog::builder()
            .modal(true)
            .transient_for(window)
            .heading(&primary_text)
            .body(&secondary_text)
            .build();

        dialog.add_responses(&[("close", &gettext("Close"))]);
        dialog.choose_future().await;
    } else {
        let notification = gio::Notification::new(&primary_text);
        notification.set_body(if secondary_text.is_empty() {
            None
        } else {
            Some(&secondary_text)
        });
        adw_app().send_notification(None, &notification);
    }
}

pub async fn confirmation_dialog(
    title: &str,
    message: &str,
    cancel: &str,
    accept: &str,
) -> Result<()> {
    ConfirmationDialog::new(title, message, cancel, accept)
        .ask()
        .await
}

pub struct ConfirmationDialog {
    title: String,
    message: String,
    cancel: String,
    accept: String,
    destructive: bool,
}

impl ConfirmationDialog {
    pub fn new(title: &str, message: &str, cancel: &str, accept: &str) -> Self {
        Self {
            title: title.to_string(),
            message: message.to_string(),
            cancel: cancel.to_string(),
            accept: accept.to_string(),
            destructive: false,
        }
    }

    pub fn set_destructive(&mut self, destructive: bool) -> &mut Self {
        self.destructive = destructive;
        self
    }

    pub async fn ask(&self) -> Result<()> {
        let dialog = adw::MessageDialog::builder()
            .transient_for(&main_ui().window())
            .modal(true)
            .heading(&self.title)
            .body(&self.message)
            .build();

        dialog.add_responses(&[("cancel", &self.cancel), ("accept", &self.accept)]);

        if self.destructive {
            dialog.set_response_appearance("accept", adw::ResponseAppearance::Destructive);
        }

        if dialog.choose_future().await == "accept" {
            Ok(())
        } else {
            Err(Error::UserCanceled)
        }
    }
}

pub fn clear(listbox: &gtk::ListBox) {
    let mut i = 0;
    while let Some(row) = listbox.row_at_index(i) {
        if !row.widget_name().starts_with('-') {
            listbox.remove(&row);
        } else {
            i += 1;
        }
    }
}

pub fn new_action_row_with_gicon(icon: Option<&gio::Icon>) -> adw::ActionRow {
    let row = adw::ActionRow::builder().activatable(true).build();

    if let Some(gicon) = icon {
        let image = gtk::Image::builder().gicon(gicon).build();

        image.add_css_class("large-row-icon");
        row.add_prefix(&image)
    }

    row
}

pub trait Logable {
    fn handle<D: Display>(&self, msg: D);
}

impl<T, E: Display> Logable for std::result::Result<T, E> {
    fn handle<D: Display>(&self, msg: D) {
        if let Err(err) = self {
            error!("Error: {}: {}", msg, err);

            let notification = gio::Notification::new(&msg.to_string());

            notification.set_body(Some(&err.to_string()));

            adw_app().send_notification(None, &notification);
        }
    }
}

/// Workaround for distros that ship(ed) xdg-desktop-portal 1.14.x, x<4
/// <https://github.com/flatpak/xdg-desktop-portal/releases/tag/1.14.4>
pub fn fix_flatpak_autostart() -> Result<()> {
    if *crate::globals::APP_IS_SANDBOXED {
        let mut path = glib::home_dir();
        path.push(".config/autostart");
        path.push(format!("{}.desktop", crate::APP_ID));

        debug!("Checking flatpak autostart file {path:?}");

        if let Ok(content) = std::fs::read_to_string(&path) {
            let new_content = content
                .replace(r"''\''", "")
                .replace(r"'\'''", "")
                .replace(r"''\\''", "")
                .replace(r"'\\'''", "");

            if new_content != content {
                warn!("Trying to fix autostart file");
                let result = std::fs::write(&path, new_content);
                match result {
                    Err(err) => {
                        return Err(
                            Message::new(gettext("Failed to fix Scheduled Backups"), err).into(),
                        );
                    }
                    Ok(()) => {
                        return Err(
                            Message::new(gettext("Scheduled Backups Repaired"), gettext("The system contained an error that potentially caused scheduled backups not to work properly. This problem is now resolved.")).into()
                        );
                    }
                }
            }
        }
    }

    Ok(())
}
