pub mod borg;
pub mod ext;
pub mod secret_service;

use gio::prelude::*;
use gtk::prelude::*;
use libhandy::prelude::*;

use crate::config;
use crate::ui::prelude::*;

use ashpd::desktop::background;
use ashpd::desktop::background::Background;
use ashpd::Response;

use std::io::Read;

/// Checks if a directory is most likely a borg repository. Performed checks are
///
/// - `data/` exists and is a directory
/// - `config` exists and contains the string "[repository]"
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

pub async fn get_background_permission() -> zbus::fdo::Result<bool> {
    let connection = zbus::Connection::new_session()?;
    let proxy = background::BackgroundProxy::new(&connection)?;

    let request_handle = proxy.request_background(
        ashpd::WindowIdentifier::default(),
        background::BackgroundOptions::default().reason(&gettext(
            "Schedule and continue running backups in background.",
        )),
    )?;

    let request = ashpd::RequestProxy::new(&connection, &request_handle)?;

    let (sender, receiver) = futures::channel::oneshot::channel();

    let sender = std::cell::Cell::new(Some(sender));

    request.on_response(|response: Response<Background>| {
        let result = match response {
            Ok(background::Background { background, .. }) => background,
            Err(err) => {
                info!("background portal: Error: {:?}", err);
                false
            }
        };

        if let Some(m) = sender.replace(None) {
            let _result = m.send(result);
        }
    })?;

    Ok(receiver.await.unwrap_or(false))
}

pub trait BackupMap<T> {
    fn get_active(&self) -> Result<&T>;
    fn get_mut_result(&mut self, key: &ConfigId) -> Result<&mut T>;
    fn get_active_mut(&mut self) -> Result<&mut T>;
    fn get_result(&self, key: &ConfigId) -> Result<&T>;
}

#[allow(clippy::implicit_hasher)]
impl<T> BackupMap<T> for std::collections::BTreeMap<ConfigId, T> {
    fn get_active(&self) -> Result<&T> {
        self.get_result(&active_config_id_result()?)
    }

    fn get_active_mut(&mut self) -> Result<&mut T> {
        self.get_mut_result(&active_config_id_result()?)
    }

    fn get_mut_result(&mut self, key: &ConfigId) -> Result<&mut T> {
        self.get_mut(&key)
            .ok_or_else(|| config::error::BackupNotFound::new(key.clone()).into())
    }

    fn get_result(&self, key: &ConfigId) -> Result<&T> {
        self.get(key)
            .ok_or_else(|| config::error::BackupNotFound::new(key.clone()).into())
    }
}

impl BackupMap<config::Backup> for config::Backups {
    fn get_active(&self) -> Result<&config::Backup> {
        self.get_result(&active_config_id_result()?)
    }

    fn get_active_mut(&mut self) -> Result<&mut config::Backup> {
        self.get_mut_result(&active_config_id_result()?)
    }

    fn get_mut_result(&mut self, key: &ConfigId) -> Result<&mut config::Backup> {
        self.0
            .iter_mut()
            .find(|x| x.id == *key)
            .ok_or_else(|| config::error::BackupNotFound::new(key.clone()).into())
    }

    fn get_result(&self, key: &ConfigId) -> Result<&config::Backup> {
        self.0
            .iter()
            .find(|x| x.id == *key)
            .ok_or_else(|| config::error::BackupNotFound::new(key.clone()).into())
    }
}

fn active_config_id_result() -> Result<ConfigId> {
    ACTIVE_BACKUP_ID
        .get()
        .ok_or_else(|| Message::short("There is no active backup in the interface.").into())
}

pub async fn spawn_thread<F, R>(
    name: &str,
    task: F,
) -> std::result::Result<R, futures::channel::oneshot::Canceled>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let (send, recv) = futures::channel::oneshot::channel();

    let sender = std::cell::Cell::new(Some(send));

    let task_name = name.to_string();
    std::thread::spawn(move || {
        if let Some(sender) = sender.replace(None) {
            if sender.send(task()).is_err() {
                error!(
                    "spawn_thread({}): Error sending to handler: Receiving end hung up",
                    task_name
                );
            }
        } else {
            error!(
                "spawn_thread({}): Error sending to handler: Already send",
                task_name
            );
        }
    });

    recv.await
}

quick_error! {
    #[derive(Debug)]
    pub enum AsyncErr {
        ThreadPanicked { display("{}", gettext("The operation terminated unexpectedly.")) }
    }
}

pub async fn folder_chooser_dialog(title: &str) -> Option<gio::File> {
    let dialog = gtk::FileChooserDialogBuilder::new()
        .title(title)
        .action(gtk::FileChooserAction::SelectFolder)
        .local_only(false)
        .transient_for(&main_ui().window())
        .modal(true)
        .build();

    dialog.add_button("_Cancel", gtk::ResponseType::Cancel);
    dialog
        .add_button("_Select", gtk::ResponseType::Accept)
        .add_css_class("suggested-action");

    let result = if dialog.run_future().await == gtk::ResponseType::Accept {
        dialog.get_file()
    } else {
        None
    };

    dialog.close();
    dialog.hide();

    result
}

pub async fn folder_chooser_dialog_path(title: &str) -> Option<std::path::PathBuf> {
    folder_chooser_dialog(title)
        .await
        .and_then(|x| x.get_path())
}

pub fn dialog_catch_err<X, P: std::fmt::Display, S: std::fmt::Display>(
    res: std::result::Result<X, P>,
    msg: S,
) -> bool {
    match res {
        Err(e) => {
            show_error(msg, e);
            true
        }
        Ok(_) => false,
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

pub fn show_error<S: std::fmt::Display, P: std::fmt::Display>(message: S, detail: P) {
    show_error_transient_for(message, detail, &main_ui().window());
}

pub fn show_error_transient_for<S: std::fmt::Display, P: std::fmt::Display, W: IsA<gtk::Window>>(
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
        let dialog = gtk::MessageDialogBuilder::new()
            .modal(true)
            .transient_for(window)
            .message_type(gtk::MessageType::Error)
            .buttons(gtk::ButtonsType::Close)
            .text(&primary_text)
            .build();

        dialog.set_property_secondary_text(if secondary_text.is_empty() {
            None
        } else {
            Some(&secondary_text)
        });

        dialog.connect_response(|dialog, _| {
            dialog.close();
            dialog.hide();
        });

        dialog.show_all();
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
) -> std::result::Result<(), UserAborted> {
    let dialog = gtk::MessageDialogBuilder::new()
        .transient_for(&main_ui().window())
        .modal(true)
        .message_type(gtk::MessageType::Question)
        .text(title)
        .secondary_text(message)
        .build();

    dialog.add_button(cancel, gtk::ResponseType::Cancel);
    dialog.add_button(accept, gtk::ResponseType::Accept);

    let result = dialog.run_future().await;
    dialog.close();
    dialog.hide();

    if result == gtk::ResponseType::Accept {
        Ok(())
    } else {
        Err(UserAborted::new())
    }
}

pub fn clear(listbox: &gtk::ListBox) {
    for c in listbox.get_children() {
        if c.get_widget_name().starts_with('-') {
            continue;
        }
        listbox.remove(&c);
    }
}

pub fn fs_usage(root: &gio::File) -> Option<(u64, u64)> {
    let none: Option<&gio::Cancellable> = None;
    if let Ok(fsinfo) = root.query_filesystem_info("*", none) {
        return Some((
            fsinfo.get_attribute_uint64(&gio::FILE_ATTRIBUTE_FILESYSTEM_SIZE),
            fsinfo.get_attribute_uint64(&gio::FILE_ATTRIBUTE_FILESYSTEM_FREE),
        ));
    }
    None
}

pub fn file_icon(path: &std::path::PathBuf, icon_size: gtk::IconSize) -> Option<gtk::Image> {
    let none: Option<&gio::Cancellable> = None;
    let file = gio::File::new_for_path(path);
    let info = file.query_info("*", gio::FileQueryInfoFlags::NONE, none);
    if let Ok(info) = info {
        let icon = info.get_icon();
        icon.map(|icon| gtk::Image::from_gicon(&icon, icon_size))
    } else {
        None
    }
}

pub fn file_symbolic_icon(
    path: &std::path::PathBuf,
    icon_size: gtk::IconSize,
) -> Option<gtk::Image> {
    let none: Option<&gio::Cancellable> = None;
    let file = gio::File::new_for_path(path);
    let info = file.query_info("*", gio::FileQueryInfoFlags::NONE, none);
    if let Ok(info) = info {
        let icon = info.get_symbolic_icon();
        icon.map(|icon| gtk::Image::from_gicon(&icon, icon_size))
    } else {
        None
    }
}

pub fn new_action_row_with_gicon(icon: Option<&gio::Icon>) -> libhandy::ActionRow {
    let row = libhandy::ActionRowBuilder::new().activatable(true).build();

    if let Some(gicon) = icon {
        row.add_prefix(&gtk::Image::from_gicon(gicon, gtk::IconSize::Dnd));
    }

    row
}
