use gio::prelude::*;
use gtk::prelude::*;
use humansize::FileSize;

use crate::borg;
use crate::shared;
use crate::ui::globals::*;
use crate::ui::prelude::*;

pub trait BackupMap<T> {
    fn get_active(&self) -> Option<&T>;
    fn get_active_mut(&mut self) -> Option<&mut T>;
}

pub fn secret_service_set_password(
    config: &shared::BackupConfig,
    password: &zeroize::Zeroizing<Vec<u8>>,
) -> Result<(), secret_service::SsError> {
    secret_service::SecretService::new(secret_service::EncryptionType::Dh)?
        .get_default_collection()?
        .create_item(
            &gettext!("{} Backup Password", crate::APPLICATION_NAME),
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
        .map(|item| item.delete())
        .collect::<Result<(), secret_service::SsError>>()
}

pub fn get_and_store_password(
    config: &shared::BackupConfig,
    pre_select_store: bool,
) -> Result<Option<shared::Password>, ()> {
    if let Some((password, store)) = crate::ui::dialog_encryption_password::Ask::new()
        .set_pre_select_store(pre_select_store)
        .run()
    {
        if store {
            debug!("Storing new password at secret service");
            if dialog_catch_err(
                secret_service_set_password(config, &password),
                gettext("Failed to store password."),
            ) {
                Ok(Some(password))
            } else {
                // Stored, so don't set password
                Ok(None)
            }
        } else {
            dialog_catch_err(
                secret_service_delete_passwords(config),
                gettext("Failed to remove potentially remaining passwords from key storage."),
            );
            Ok(Some(password))
        }
    } else {
        // User aborted
        Err(())
    }
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

pub struct Async(());

impl Async {
    pub fn borg<F, G, V>(name: &'static str, borg: borg::Borg, task: F, result_handler: G)
    where
        F: FnOnce(borg::Borg) -> Result<V, shared::BorgErr> + Send + Clone + 'static,
        G: Fn(Result<V, shared::BorgErr>) + Clone + 'static,
        V: Send + 'static,
    {
        borg_async(name, borg, task, result_handler, false)
    }
}

fn borg_async<F, G, V>(
    name: &'static str,
    borg: borg::Borg,
    task: F,
    result_handler: G,
    pre_select_store: bool,
) where
    F: FnOnce(borg::Borg) -> Result<V, shared::BorgErr> + Send + Clone + 'static,
    G: Fn(Result<V, shared::BorgErr>) + Clone + 'static,
    V: Send + 'static,
{
    async_react(
        name,
        enclose!((borg, task) move || task(borg)),
        move |result| {
            if let Err(ref err) = result {
                if matches!(err, shared::BorgErr::PasswordMissing)
                    || err.has_borg_msgid(&shared::MsgId::PassphraseWrong)
                {
                    let mut borg = borg.clone();
                    if let Ok(ask_password) =
                        get_and_store_password(&borg.get_config(), pre_select_store)
                    {
                        if let Some(ref password) = ask_password {
                            borg.set_password(password.clone());
                        } else {
                            borg.unset_password();
                        }
                        borg_async(
                            name,
                            borg,
                            task.clone(),
                            result_handler.clone(),
                            ask_password.is_none(),
                        );
                    } else {
                        result_handler(Err(shared::BorgErr::UserAborted))
                    }
                    return;
                }
            }

            result_handler(result);
        },
    );
}

/// Calls the result handler if the task has returned
pub fn async_react<F, G, R>(name: &str, task: F, result_handler: G)
where
    F: FnOnce() -> R + Send + 'static,
    G: Fn(R) + 'static,
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
            result_handler(result);
            Continue(false)
        }
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            error!("async_react({}): Task disconnected", task_name);
            Continue(false)
        }
        Err(std::sync::mpsc::TryRecvError::Empty) => Continue(true),
    });
}

pub fn hsize(bytes: u64) -> String {
    bytes
        .file_size(humansize::file_size_opts::CONVENTIONAL)
        .unwrap_or_default()
}

pub fn hsized(bytes: u64, decimal_places: usize) -> String {
    let mut opts = humansize::file_size_opts::CONVENTIONAL;
    opts.decimal_places = decimal_places;
    bytes.file_size(opts).unwrap_or_default()
}

pub trait WidgetEnh {
    fn add_css_class(&self, class: &str);
    fn remove_css_class(&self, class: &str);
}

impl<W: gtk::WidgetExt> WidgetEnh for W {
    fn add_css_class(&self, class: &str) {
        self.get_style_context().add_class(class);
    }

    fn remove_css_class(&self, class: &str) {
        self.get_style_context().remove_class(class);
    }
}

pub fn folder_chooser_dialog(title: &str) -> Option<std::path::PathBuf> {
    let dialog = gtk::FileChooserDialog::with_buttons(
        Some(title),
        Some(&main_ui().window()),
        gtk::FileChooserAction::SelectFolder,
        &[
            ("_Cancel", gtk::ResponseType::Cancel),
            ("_Select", gtk::ResponseType::Accept),
        ],
    );

    if let Some(button) = dialog.get_widget_for_response(gtk::ResponseType::Accept) {
        button.add_css_class("suggested-action");
    }

    let result = if dialog.run() == gtk::ResponseType::Accept {
        dialog.get_filename()
    } else {
        None
    };

    dialog.close();
    dialog.hide();

    result
}

pub fn dialog_catch_errb<X, P, S: AsRef<str>>(res: &Result<X, P>, msg: S) -> bool
where
    P: std::fmt::Display,
{
    match res {
        Err(e) => {
            dialog_error(&format!("{}\n\n{}", msg.as_ref(), e));
            true
        }
        Ok(_) => false,
    }
}

pub fn dialog_catch_err<X, P, S: AsRef<str>>(res: Result<X, P>, msg: S) -> bool
where
    P: std::fmt::Display,
{
    match res {
        Err(e) => {
            let formatted = format!("{}\n\n{}", msg.as_ref(), e);
            warn!("Displaying caught error:\n\n{}", formatted);
            dialog_error(&formatted);
            true
        }
        Ok(_) => false,
    }
}

pub fn dialog_error<S: AsRef<str>>(error: S) {
    let error_vec = error.as_ref().chars().collect::<Vec<_>>();

    let mut error_str = error.as_ref().to_string();

    if error_vec.len() > 400 {
        error_str = format!(
            "{}\n…\n{}",
            error_vec.iter().take(200).collect::<String>(),
            error_vec.iter().rev().take(200).rev().collect::<String>()
        );
    }

    let dialog = gtk::MessageDialog::new(
        Some(&main_ui().window()),
        gtk::DialogFlags::MODAL,
        gtk::MessageType::Error,
        gtk::ButtonsType::Ok,
        &error_str,
    );

    dialog.run();
    dialog.close();
    dialog.hide();
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

pub fn add_list_box_row(
    list_box: &gtk::ListBox,
    name: Option<&str>,
    position: i32,
) -> (gtk::ListBoxRow, gtk::Box) {
    let row = gtk::ListBoxRow::new();
    list_box.insert(&row, position);

    let horizontal_box = gtk::Box::new(gtk::Orientation::Horizontal, 18);
    row.add(&horizontal_box);

    if let Some(name) = name {
        row.set_widget_name(name);
    }

    (row, horizontal_box)
}

pub fn list_vertical_box(
    text1: Option<&str>,
    text2: Option<&str>,
) -> (gtk::Box, gtk::Label, gtk::Label) {
    let vertical_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    vertical_box.set_valign(gtk::Align::Center);

    let label1 = gtk::Label::new(text1);
    let label2 = gtk::Label::new(text2);

    vertical_box.add(&label1);
    vertical_box.add(&label2);

    label1.set_halign(gtk::Align::Start);
    label1.set_ellipsize(pango::EllipsizeMode::End);
    label2.set_halign(gtk::Align::Start);
    label2.set_ellipsize(pango::EllipsizeMode::End);

    label2.add_css_class("dim-label");
    label2.add_css_class("small-label");

    (vertical_box, label1, label2)
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

pub fn repo_icon(repo: &shared::BackupRepo) -> String {
    match repo {
        shared::BackupRepo::Local { icon, .. } => {
            icon.clone().unwrap_or_else(|| String::from("folder"))
        }
        shared::BackupRepo::Remote { .. } => String::from("network-server"),
    }
}
