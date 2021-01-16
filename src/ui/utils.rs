use gio::prelude::*;
use gtk::prelude::*;
use libhandy::prelude::*;

use crate::borg;
use crate::shared::{self, Password};
use crate::ui::globals::*;
use crate::ui::prelude::*;

pub trait BackupMap<T> {
    fn get_active(&self) -> Option<&T>;
    fn get_active_mut(&mut self) -> Option<&mut T>;
}

pub fn secret_service_set_password(
    config: &shared::BackupConfig,
    password: &Password,
) -> Result<(), secret_service::Error> {
    secret_service::SecretService::new(secret_service::EncryptionType::Dh)?
        .get_default_collection()?
        .create_item(
            // Translators: This is the description for entries in the password database.
            &gettext("Pika Backup Password"),
            [
                ("backup_id", config.id.as_str()),
                ("program", env!("CARGO_PKG_NAME")),
            ]
            .iter()
            .cloned()
            .collect(),
            password,
            true,
            "text/plain",
        )?;

    Ok(())
}

pub fn secret_service_delete_passwords(
    config: &shared::BackupConfig,
) -> Result<(), secret_service::Error> {
    secret_service::SecretService::new(secret_service::EncryptionType::Dh)?
        .get_default_collection()?
        .search_items(
            [
                ("backup_id", config.id.as_str()),
                ("program", env!("CARGO_PKG_NAME")),
            ]
            .iter()
            .cloned()
            .collect(),
        )?
        .iter()
        .try_for_each(|item| item.delete())
}

pub async fn get_password(pre_select_store: bool) -> Option<(shared::Password, bool)> {
    crate::ui::dialog_encryption_password::Ask::new()
        .set_pre_select_store(pre_select_store)
        .run()
        .await
}

#[allow(clippy::implicit_hasher)]
impl<T> BackupMap<T> for std::collections::HashMap<String, T> {
    fn get_active(&self) -> Option<&T> {
        self.get(&ACTIVE_BACKUP_ID.get()?)
    }
    fn get_active_mut(&mut self) -> Option<&mut T> {
        self.get_mut(&ACTIVE_BACKUP_ID.get()?)
    }
}

#[allow(clippy::implicit_hasher)]
impl<T> BackupMap<T> for std::collections::BTreeMap<String, T> {
    fn get_active(&self) -> Option<&T> {
        self.get(&ACTIVE_BACKUP_ID.get()?)
    }
    fn get_active_mut(&mut self) -> Option<&mut T> {
        self.get_mut(&ACTIVE_BACKUP_ID.get()?)
    }
}

pub fn store_password(config: &shared::BackupConfig, x: &Option<(Password, bool)>) {
    if let Some((ref password, ref store)) = x {
        if *store {
            debug!("Storing new password at secret service");
            dialog_catch_err(
                secret_service_set_password(&config, &password),
                gettext("Failed to store password."),
            );
        } else {
            debug!("Removing password from secret service");
            dialog_catch_err(
                secret_service_delete_passwords(config),
                gettext("Failed to remove potentially remaining passwords from key storage."),
            );
        }
    }
}

pub struct Async(());

impl Async {
    pub async fn borg_spawn<F, V>(
        name: &'static str,
        borg: borg::Borg,
        task: F,
    ) -> Result<V, shared::BorgErr>
    where
        F: FnOnce(borg::Borg) -> Result<V, shared::BorgErr> + Send + Clone + 'static + Sync,
        V: Send + 'static,
    {
        let config = borg.get_config();

        let result = borg_spawn(name, borg, task, false).await;

        if let Ok((_, ref x)) = result {
            store_password(&config, x);
        }
        result.map(|(x, _)| x)
    }

    pub async fn borg_only_repo_suggest_store<F, V, B>(
        name: &'static str,
        borg: B,
        task: F,
    ) -> Result<(V, Option<(Password, bool)>), shared::BorgErr>
    where
        F: FnOnce(B) -> Result<V, shared::BorgErr> + Send + Clone + 'static + Sync,
        V: Send + 'static,
        B: borg::BorgBasics + 'static,
    {
        borg_spawn(name, borg, task, true).await
    }
}

#[allow(clippy::type_complexity)]
async fn borg_spawn<F, V, B>(
    name: &'static str,
    mut borg: B,
    task: F,
    mut pre_select_store: bool,
) -> Result<(V, Option<(Password, bool)>), shared::BorgErr>
where
    F: FnOnce(B) -> Result<V, shared::BorgErr> + Send + Clone + 'static + Sync,
    V: Send + 'static,
    B: borg::BorgBasics + 'static,
{
    loop {
        let result = spawn_thread(
            name,
            enclose!((borg, task)
         move || task(borg)),
        )
        .await;

        return match result {
            Err(futures::channel::oneshot::Canceled) => Err(shared::BorgErr::ThreadPanicked),
            Ok(result) => match result {
                Err(e)
                    if matches!(e, shared::BorgErr::PasswordMissing)
                        || e.has_borg_msgid(&shared::MsgId::PassphraseWrong) =>
                {
                    if let Some((password, store)) = get_password(pre_select_store).await {
                        pre_select_store = store;
                        borg.set_password(password);

                        continue;
                    } else {
                        Err(shared::BorgErr::UserAborted)
                    }
                }
                Err(e) => Err(e),
                Ok(result) => Ok((result, borg.get_password().map(|p| (p, pre_select_store)))),
            },
        };
    }
}

pub async fn spawn_thread<F, R>(
    name: &str,
    task: F,
) -> Result<R, futures::channel::oneshot::Canceled>
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
                "spawn_thread({}): Error sending to handler: Allready send",
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
    res: Result<X, P>,
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

    warn!(
        "Displaying caught error:\n{}\n{}",
        &primary_text, &secondary_text
    );

    dialog.show_all();
}

pub fn dialog_error<S: std::fmt::Display>(error: S) {
    show_error(error, "");
}

pub async fn confirmation_dialog(title: &str, message: &str, cancel: &str, accept: &str) -> bool {
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

    result == gtk::ResponseType::Accept
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
