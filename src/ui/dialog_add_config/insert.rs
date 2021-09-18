use std::rc::Rc;

use gio::prelude::*;
use gtk::prelude::*;
use zeroize::Zeroizing;

use crate::borg;
use crate::borg::prelude::*;
use crate::config;
use crate::config::*;
use crate::ui;
use crate::ui::builder;
use crate::ui::prelude::*;
use ui::page_pending;

pub fn on_init_button_clicked(ui: Rc<builder::DialogAddConfig>) {
    execute(on_init_button_clicked_future(Rc::clone(&ui)), ui.dialog());
}

pub fn on_add_button_clicked(ui: Rc<builder::DialogAddConfig>) {
    execute(on_add_button_clicked_future(ui.clone()), ui.dialog());
}

pub fn on_add_repo_list_activated(row: Rc<gtk::ListBoxRow>, ui: Rc<builder::DialogAddConfig>) {
    let name = row.widget_name();
    if name == "-add-local" {
        execute(on_add_repo_list_activated_local(ui.clone()), ui.dialog());
    } else if name == "-add-remote" {
        ui.stack().set_visible_child(&ui.new_page());
        ui.location_stack().set_visible_child(&ui.location_remote());
        ui.button_stack().set_visible_child(&ui.add_button());
        ui.encryption_box().hide();
        ui.add_button().show();
        ui.add_button().grab_default();
    } else {
        page_pending::show(&gettext("Loading backup repository"));
        ui.dialog().hide();

        let uri = name;
        if let Some(path) = gio::File::for_uri(&uri).path() {
            execute(
                add_repo_config(local::Repository::from_path(path).into_config(), ui.clone()),
                ui.dialog(),
            );
        }
    }
}

async fn on_add_repo_list_activated_local(ui: Rc<builder::DialogAddConfig>) -> Result<()> {
    ui.dialog().hide();

    if let Some(path) = ui::utils::folder_chooser_dialog(&gettext("Select existing repository"))
        .await
        .and_then(|x| x.path())
    {
        ui::page_pending::show(&gettext("Loading backup repository"));
        if ui::utils::is_backup_repo(&path) {
            add_repo_config(local::Repository::from_path(path).into_config(), ui).await?;
        } else {
            return Err(Message::new(
                gettext("Location is not a valid backup repository."),
                gettext("The repository must originate from Pika Backup or compatible software."),
            )
            .into());
        }
    } else {
        ui.dialog().show();
    }

    Ok(())
}

async fn on_add_button_clicked_future(ui: Rc<builder::DialogAddConfig>) -> Result<()> {
    page_pending::show(&gettext("Loading backup repository"));
    ui.dialog().hide();

    let url = ui.location_url().text();
    let file = gio::File::for_uri(&url);
    debug!("Add existing URI '{:?}'", file.path());

    let repo = if url.get(..6) == Some("ssh://") {
        config::remote::Repository::from_uri(url.to_string()).into_config()
    } else {
        mount_fuse_and_config(&file, false).await?.into_config()
    };

    add_repo_config(repo, ui).await
}

async fn on_init_button_clicked_future(ui: Rc<builder::DialogAddConfig>) -> Result<()> {
    let encrypted =
        ui.encryption().visible_child() != Some(ui.unencrypted().upcast::<gtk::Widget>());

    if encrypted {
        if ui.password().text().is_empty() {
            return Err(Message::new(
                gettext("No password provided."),
                gettext("To use encryption a password must be provided."),
            )
            .into());
        } else if ui.password().text() != ui.password_confirm().text() {
            return Err(Message::short(gettext("Entered passwords do not match.")).into());
        }
    }

    let mut repo = if ui.location_stack().visible_child()
        == Some(ui.location_local().upcast::<gtk::Widget>())
    {
        if let Some(path) = ui
            .init_path()
            .file()
            .map(|x| x.child(ui.init_dir().text().as_str()))
            .and_then(|x| x.path())
        {
            if let Some(mount) = ui.init_path().file().and_then(|file| {
                file.find_enclosing_mount(Some(&gio::Cancellable::new()))
                    .ok()
            }) {
                let uri = gio::File::for_path(&path).uri().to_string();
                Ok(local::Repository::from_mount(mount, path, uri).into_config())
            } else {
                Ok(local::Repository::from_path(path).into_config())
            }
        } else {
            Err(Message::short(gettext("A repository location has to be given.")).into())
        }
    } else {
        let url = ui.location_url().text().to_string();
        let file = gio::File::for_uri(&ui.location_url().text());

        if url.is_empty() {
            Err(Message::short(gettext("A repository location has to be given.")).into())
        } else if url.get(..6) == Some("ssh://") {
            Ok(config::remote::Repository::from_uri(url).into_config())
        } else {
            mount_fuse_and_config(&file, true)
                .await
                .map(|x| x.into_config())
        }
    }?;

    if let Ok(args) = command_line_args(&ui) {
        repo.set_settings(Some(BackupSettings {
            command_line_args: Some(args),
        }));
    } else {
        return Err(Message::new(
            gettext("Additional command line arguments invalid."),
            gettext("Please check for missing closing quotes."),
        )
        .into());
    }

    page_pending::show(&gettext("Creating backup repository"));
    ui.dialog().hide();

    let mut borg = borg::BorgOnlyRepo::new(repo.clone());
    let password = Zeroizing::new(ui.password().text().as_bytes().to_vec());
    if encrypted {
        borg.set_password(password.clone());
    }

    let result =
        ui::utils::spawn_thread(&gettext("Creating backup repository"), move || borg.init()).await;

    match result.unwrap_or(Err(borg::Error::ThreadPanicked)) {
        Err(err) => {
            return Err(Message::new(gettext("Failed to initialize repository."), err).into());
        }
        Ok(info) => {
            let config = config::Backup::new(repo.clone(), info, encrypted);

            insert_backup_config(config.clone())?;
            if encrypted && ui.password_store().is_active() {
                if let Err(err) = ui::utils::secret_service::set_password(&config, &password) {
                    return Err(Message::new(gettext("Failed to store password."), err).into());
                }
            }
            ui::page_backup::view_backup_conf(&config.id);
        }
    };

    Ok(())
}

async fn add_repo_config(
    mut repo: config::Repository,
    ui: Rc<builder::DialogAddConfig>,
) -> Result<()> {
    repo.set_settings(Some(BackupSettings {
        command_line_args: Some(command_line_args(&ui)?),
    }));

    insert_backup_config_encryption_unknown(repo).await
}

async fn insert_backup_config_encryption_unknown(repo: config::Repository) -> Result<()> {
    let (info, pw_data) = ui::utils::borg::only_repo_suggest_store(
        &gettext("Loading backup repository"),
        borg::BorgOnlyRepo::new(repo.clone()),
        |borg| borg.peek(),
    )
    .await
    .into_message("Failed to configure repository.")?;

    let encrypted = pw_data
        .clone()
        .map(|(password, _)| !password.is_empty())
        .unwrap_or_default();
    let config = config::Backup::new(repo.clone(), info, encrypted);
    insert_backup_config(config.clone())?;
    ui::page_backup::view_backup_conf(&config.id);
    ui::utils::secret_service::store_password(&config, &pw_data)?;

    Ok(())
}

fn insert_backup_config(config: config::Backup) -> Result<()> {
    BACKUP_CONFIG.update_result(move |s| {
        s.insert(config.clone())?;
        Ok(())
    })?;

    ui::write_config()
}

fn execute<
    F: std::future::Future<Output = Result<()>> + 'static,
    W: IsA<gtk::Window> + IsA<gtk::Widget>,
>(
    f: F,
    window: W,
) {
    Handler::new()
        .error_transient_for(window.clone())
        .dialog_auto_visibility(window)
        .spawn(f);
}

fn command_line_args(ui: &builder::DialogAddConfig) -> Result<Vec<String>> {
    if let Ok(args) = shell_words::split(
        &ui.command_line_args()
            .buffer()
            .and_then(|buffer| {
                let (start, end) = buffer.bounds();
                buffer.text(&start, &end, false).map(|x| x.to_string())
            })
            .unwrap_or_default(),
    ) {
        Ok(args)
    } else {
        Err(Message::new(
            gettext("Additional command line arguments invalid."),
            gettext("Please check for missing closing quotes."),
        )
        .into())
    }
}

async fn mount_fuse_and_config(file: &gio::File, mount_parent: bool) -> Result<local::Repository> {
    if let (Ok(mount), Some(path)) = (
        file.find_enclosing_mount(Some(&gio::Cancellable::new())),
        file.path(),
    ) {
        Ok(local::Repository::from_mount(
            mount,
            path,
            file.uri().to_string(),
        ))
    } else {
        let mount_uri = if mount_parent {
            file.parent().as_ref().unwrap_or(file).uri()
        } else {
            file.uri()
        };

        ui::dialog_device_missing::mount_enclosing(&gio::File::for_uri(&mount_uri)).await?;

        if let (Ok(mount), Some(path)) = (
            file.find_enclosing_mount(Some(&gio::Cancellable::new())),
            file.path(),
        ) {
            Ok(local::Repository::from_mount(
                mount,
                path,
                file.uri().to_string(),
            ))
        } else {
            Err(Error::Message(Message::new(
                gettext("Repository location not found."),
                gettext("A mount operation succeeded but the location is still unavailable."),
            )))
        }
    }
}
