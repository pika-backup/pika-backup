pub mod borg;
pub mod config_io;
pub mod df;
pub mod duration;
pub mod ext;
pub mod repo_cache;
pub mod secret_service;

use crate::ui::prelude::*;
use adw::prelude::*;

use crate::config;

use ashpd::desktop::background;
use std::fmt::Display;
use std::io::Read;
use std::os::unix::process::CommandExt;

pub struct StatusRow {
    pub title: String,
    pub subtitle: String,
    pub icon_name: String,
    pub level: StatusLevel,
}

#[derive(Clone, Copy)]
pub enum StatusLevel {
    Ok,
    Neutral,
    Warning,
    Error,
}

impl StatusRow {
    pub fn action_row(&self) -> adw::ActionRow {
        let icon = super::widget::StatusIcon::new(&self.icon_name, self.level);

        let row = adw::ActionRow::builder()
            .title(&self.title)
            .subtitle(&self.subtitle)
            .build();

        row.add_prefix(&icon);

        row
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
use std::convert::TryInto;
pub async fn background_permission() -> Result<()> {
    let generic_msg = gettext("Request to run in background failed");

    if !ashpd::is_sandboxed() {
        // start
        let proxy = zbus::fdo::DBusProxy::new(&ZBUS_SESSION)
            .await
            .err_to_msg(&generic_msg)?;

        proxy
            .start_service_by_name(crate::DAEMON_APP_ID.try_into().unwrap(), Default::default())
            .await
            .err_to_msg(&generic_msg)?;

        // without flatpak we can always run in background
        Ok(())
    } else {
        let response = background::request(
            &ashpd::WindowIdentifier::default(),
            &gettext("Schedule backups and continue running backups."),
            true,
            Some(&[crate::DAEMON_BINARY]),
            // Do not use dbus-activation because that would start the UI
            // See <https://gitlab.gnome.org/Teams/Design/hig-www/-/issues/107>
            false,
        )
        .await;

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
                        gettext("Either the system does not support this feature or an error occurred. Scheduled backup functionality and continuing backups in the background will not be available.")
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

impl<C: LookupActiveConfigId> LookupActiveConfigId for config::Writeable<C> {
    type Item = C::Item;

    fn active(&self) -> Result<&Self::Item> {
        self.current_config.active()
    }

    fn active_mut(&mut self) -> Result<&mut Self::Item> {
        self.current_config.active_mut()
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
        .accept_label(&gettext("Select"))
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
        Err(UserCanceled::new().into())
    } else {
        Ok(paths)
    }
}

fn ellipsize<S: std::fmt::Display>(x: S) -> String {
    let s = x.to_string();
    let vec = s.chars().collect::<Vec<_>>();

    if vec.len() > 410 {
        format!(
            "{}\n…\n{}",
            vec.iter().take(200).collect::<String>(),
            vec.iter().rev().take(200).rev().collect::<String>()
        )
    } else {
        s
    }
}

pub fn show_notice<S: std::fmt::Display>(message: S) {
    warn!("Displaying notice:\n  {}", message);

    let toast = adw::Toast::builder()
        .title(&message.to_string())
        .timeout(0)
        .build();

    main_ui().toast().add_toast(&toast);

    if !crate::ui::app_window::is_displayed() {
        let notification = gio::Notification::new(&message.to_string());
        adw_app().send_notification(None, &notification);
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
    let primary_text = ellipsize(message);
    let secondary_text = ellipsize(detail);
    warn!(
        "Displaying error:\n  {}\n  {}",
        &primary_text, &secondary_text
    );

    if crate::ui::app_window::is_displayed() {
        let dialog = adw::MessageDialog::builder()
            .modal(true)
            .transient_for(window)
            .heading(&primary_text)
            .body(&secondary_text)
            .build();

        dialog.add_responses(&[("close", &gettext("Close"))]);
        dialog.run_future().await;

        dialog.close();
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
) -> std::result::Result<(), UserCanceled> {
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

    pub async fn ask(&self) -> std::result::Result<(), UserCanceled> {
        let dialog = adw::MessageDialog::builder()
            .transient_for(&main_ui().window())
            .modal(true)
            .heading(&self.title)
            .body(&self.message)
            .build();

        dialog.add_responses(&[("cancel", &self.cancel), ("accept", &self.accept)]);

        if self.destructive {
            dialog.set_response_appearance("replace", adw::ResponseAppearance::Destructive);
        }

        let result = dialog.run_future().await;
        dialog.destroy();

        if result == "accept" {
            Ok(())
        } else {
            Err(UserCanceled::new())
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
        row.add_prefix(
            &gtk::Image::builder()
                .gicon(gicon)
                .css_classes(vec![String::from("large-row-icon")])
                .build(),
        );
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
    if ashpd::is_sandboxed() {
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
