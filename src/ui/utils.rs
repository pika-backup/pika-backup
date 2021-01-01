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
) -> Result<(), secret_service::SsError> {
    secret_service::SecretService::new(secret_service::EncryptionType::Dh)?
        .get_default_collection()?
        .create_item(
            // Translators: This is the description for entries in the password database.
            &gettext("Pika Backup Password"),
            vec![
                ("backup_id", &config.id),
                ("program", env!("CARGO_PKG_NAME")),
            ],
            password,
            true,
            "text/plain",
        )?;

    Ok(())
}

pub fn secret_service_delete_passwords(
    config: &shared::BackupConfig,
) -> Result<(), secret_service::SsError> {
    secret_service::SecretService::new(secret_service::EncryptionType::Dh)?
        .get_default_collection()?
        .search_items(vec![
            ("backup_id", &config.id),
            ("program", env!("CARGO_PKG_NAME")),
        ])?
        .iter()
        .try_for_each(|item| item.delete())
}

pub fn get_password(pre_select_store: bool) -> Option<(shared::Password, bool)> {
    crate::ui::dialog_encryption_password::Ask::new()
        .set_pre_select_store(pre_select_store)
        .run()
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
    pub fn borg<F, G, V>(name: &'static str, borg: borg::Borg, task: F, result_handler: G)
    where
        F: FnOnce(borg::Borg) -> Result<V, shared::BorgErr> + Send + Clone + 'static,
        G: Fn(Result<V, shared::BorgErr>) + Clone + 'static,
        V: Send + 'static,
    {
        let config = borg.get_config();

        borg_async(
            name,
            borg,
            task,
            move |result| {
                if let Ok((_, ref x)) = result {
                    store_password(&config, x);
                }
                result_handler(result.map(|(x, _)| x));
            },
            false,
        )
    }

    pub fn borg_only_repo_suggest_store<F, G, V, B>(
        name: &'static str,
        borg: B,
        task: F,
        result_handler: G,
    ) where
        F: FnOnce(B) -> Result<V, shared::BorgErr> + Send + Clone + 'static,
        G: Fn(Result<(V, Option<(Password, bool)>), shared::BorgErr>) + Clone + 'static,
        V: Send + 'static,
        B: borg::BorgBasics + 'static,
    {
        borg_async(name, borg, task, result_handler, true)
    }
}

fn borg_async<F, G, V, B>(
    name: &'static str,
    borg: B,
    task: F,
    result_handler: G,
    pre_select_store: bool,
) where
    F: FnOnce(B) -> Result<V, shared::BorgErr> + Send + Clone + 'static,
    G: Fn(Result<(V, Option<(Password, bool)>), shared::BorgErr>) + Clone + 'static,
    V: Send + 'static,
    B: borg::BorgBasics + 'static,
{
    async_react(
        name,
        enclose!((borg, task)
         move || task(borg)),
        move |result| match result {
            Err(AsyncErr::ThreadPanicked) => result_handler(Err(shared::BorgErr::ThreadPanicked)),
            Ok(result) => match result {
                Err(e)
                    if matches!(e, shared::BorgErr::PasswordMissing)
                        || e.has_borg_msgid(&shared::MsgId::PassphraseWrong) =>
                {
                    let mut borg = borg.clone();
                    if let Some((password, store)) = get_password(pre_select_store) {
                        borg.set_password(password);

                        borg_async(name, borg, task.clone(), result_handler.clone(), store);
                    } else {
                        result_handler(Err(shared::BorgErr::UserAborted))
                    }
                }
                Err(e) => result_handler(Err(e)),
                Ok(result) => result_handler(Ok((
                    result,
                    borg.get_password().map(|p| (p, pre_select_store)),
                ))),
            },
        },
    );
}

/// Calls the result handler if the task has returned
pub fn async_react<F, G, R>(name: &str, task: F, result_handler: G)
where
    F: FnOnce() -> R + Send + 'static,
    G: Fn(Result<R, AsyncErr>) + 'static,
    R: Send + 'static,
{
    let (send, recv) = std::sync::mpsc::channel();

    let task_name = name.to_string();
    std::thread::spawn(move || {
        send.send(task()).unwrap_or_else(|e| {
            error!(
                "async_react({}): Error sending to handler: {}",
                task_name, e
            );
        });
    });

    let task_name = name.to_string();
    glib::timeout_add_local(50, move || match recv.try_recv() {
        Ok(result) => {
            result_handler(Ok(result));
            Continue(false)
        }
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            error!("async_react({}): Task disconnected", task_name);
            result_handler(Err(AsyncErr::ThreadPanicked));
            Continue(false)
        }
        Err(std::sync::mpsc::TryRecvError::Empty) => Continue(true),
    });
}

quick_error! {
    #[derive(Debug)]
    pub enum AsyncErr {
        ThreadPanicked { display("{}", gettext("The responsible thread has panicked")) }
    }
}

pub fn folder_chooser_dialog(title: &str) -> Option<gio::File> {
    let dialog = gtk::FileChooserDialog::with_buttons(
        Some(title),
        Some(&main_ui().window()),
        gtk::FileChooserAction::SelectFolder,
        &[
            ("_Cancel", gtk::ResponseType::Cancel),
            ("_Select", gtk::ResponseType::Accept),
        ],
    );

    dialog.set_local_only(false);

    if let Some(button) = dialog.get_widget_for_response(gtk::ResponseType::Accept) {
        button.add_css_class("suggested-action");
    }

    let result = if dialog.run() == gtk::ResponseType::Accept {
        dialog.get_file()
    } else {
        None
    };

    dialog.close();
    dialog.hide();

    result
}

pub fn folder_chooser_dialog_path(title: &str) -> Option<std::path::PathBuf> {
    folder_chooser_dialog(title).and_then(|x| x.get_path())
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
    let primary_text = ellipsize(message);
    let secondary_text = ellipsize(detail);

    let dialog = gtk::MessageDialog::new(
        Some(&main_ui().window()),
        gtk::DialogFlags::MODAL,
        gtk::MessageType::Error,
        gtk::ButtonsType::Ok,
        &primary_text,
    );

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

    dialog.run();
}

pub fn dialog_error<S: std::fmt::Display>(error: S) {
    show_error(error, "");
}

pub fn dialog_yes_no<S: AsRef<str>>(message: S) -> bool {
    let dialog = gtk::MessageDialog::new(
        Some(&main_ui().window()),
        gtk::DialogFlags::MODAL,
        gtk::MessageType::Question,
        gtk::ButtonsType::YesNo,
        message.as_ref(),
    );

    let result = dialog.run() == gtk::ResponseType::Yes;
    dialog.close();
    dialog.hide();
    result
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
    let row = libhandy::ActionRow::new();
    row.set_activatable(true);

    if let Some(gicon) = icon {
        row.add_prefix(&gtk::Image::from_gicon(gicon, gtk::IconSize::Dnd));
    }

    row
}
