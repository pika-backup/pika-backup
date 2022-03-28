pub mod borg;
pub mod df;
pub mod duration;
pub mod ext;
pub mod repo_cache;
pub mod secret_service;

use adw::prelude::*;

use crate::config;
use crate::ui::prelude::*;

use ashpd::desktop::background;

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
        let icon = super::widgets::StatusIcon::new(&self.icon_name, self.level);

        let row = adw::ActionRow::builder()
            .title(&self.title)
            .subtitle(&self.subtitle)
            .build();

        row.add_prefix(&icon);

        row
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
                        #[allow(unused_must_use)]
                        {
                            cfg.read_to_string(&mut content);
                        }
                        return content.contains("[repository]");
                    }
                }
            }
        }
    };

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
            .start_service_by_name(
                crate::daemon_app_id().try_into().unwrap(),
                Default::default(),
            )
            .await
            .err_to_msg(&generic_msg)?;

        // without flatpak we can always run in background
        Ok(())
    } else {
        let response = background::request(
            &ashpd::WindowIdentifier::default(),
            &gettext("Schedule backups and continue running backups."),
            true,
            Some(&["pika-backup-monitor"]),
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
                let mut command = std::process::Command::new("pika-backup-monitor");

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
                    .err_to_msg(gettext("Failed to start background process"))
                    .map(|_| ())
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

    let result = if dialog.run_future().await == gtk::ResponseType::Accept {
        dialog.file()
    } else {
        None
    };

    result
}

pub async fn folder_chooser_dialog_path(title: &str) -> Option<std::path::PathBuf> {
    folder_chooser_dialog(title).await.and_then(|x| x.path())
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
        gtk_app().send_notification(None, &notification);
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
        let dialog = gtk::MessageDialog::builder()
            .modal(true)
            .transient_for(window)
            .message_type(gtk::MessageType::Error)
            .buttons(gtk::ButtonsType::Close)
            .text(&primary_text)
            .build();

        dialog.set_secondary_text(if secondary_text.is_empty() {
            None
        } else {
            Some(&secondary_text)
        });

        dialog.run_future().await;

        dialog.close();
    } else {
        let notification = gio::Notification::new(&primary_text);
        notification.set_body(if secondary_text.is_empty() {
            None
        } else {
            Some(&secondary_text)
        });
        gtk_app().send_notification(None, &notification);
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
        let dialog = gtk::MessageDialog::builder()
            .transient_for(&main_ui().window())
            .modal(true)
            .message_type(gtk::MessageType::Question)
            .text(&self.title)
            .secondary_text(&self.message)
            .build();

        dialog.add_button(&self.cancel, gtk::ResponseType::Cancel);
        let accept_button = dialog.add_button(&self.accept, gtk::ResponseType::Accept);

        if self.destructive {
            accept_button.add_css_class("destructive-action");
        }

        let result = dialog.run_future().await;
        dialog.close();
        dialog.hide();

        if result == gtk::ResponseType::Accept {
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

pub fn file_icon(path: &std::path::Path) -> Option<gtk::Image> {
    let file = gio::File::for_path(path);
    let info = file.query_info(
        "*",
        gio::FileQueryInfoFlags::NONE,
        gtk::gio::Cancellable::NONE,
    );

    let mut gicon = None;
    if let Ok(info) = info {
        if let Some(icon) = info.icon() {
            gicon = Some(icon.clone());

            if let Some(theme) =
                gtk::gdk::Display::default().map(|d| gtk::IconTheme::for_display(&d))
            {
                if !theme.has_gicon(&icon) {
                    gicon = gio::Icon::for_string("system-run").ok();
                }
            }
        }
    }

    if gicon.is_none() {
        gicon = gio::Icon::for_string("folder-visiting").ok();
    }

    gicon.map(|x| {
        gtk::Image::builder()
            .gicon(&x)
            .css_classes(vec![String::from("row-icon")])
            .build()
    })
}

pub fn file_symbolic_icon(path: &std::path::Path) -> Option<gtk::Image> {
    let file = gio::File::for_path(path);
    let info = file.query_info(
        "*",
        gio::FileQueryInfoFlags::NONE,
        gtk::gio::Cancellable::NONE,
    );
    if let Ok(info) = info {
        let icon = info.symbolic_icon();
        icon.map(|icon| gtk::Image::from_gicon(&icon))
    } else {
        None
    }
}

pub fn new_action_row_with_gicon(icon: Option<&gio::Icon>) -> adw::ActionRow {
    let row = adw::ActionRow::builder().activatable(true).build();

    if let Some(gicon) = icon {
        row.add_prefix(
            &gtk::Image::builder()
                .gicon(gicon)
                .css_classes(vec![String::from("row-icon")])
                .build(),
        );
    }

    row
}
