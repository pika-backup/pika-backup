use adw::prelude::*;

use super::display;
use super::remote_location::RemoteLocation;
use crate::borg;
use crate::borg::prelude::*;
use crate::config;
use crate::config::*;
use crate::ui;
use crate::ui::builder;
use crate::ui::prelude::*;

pub async fn on_add_repo_list_activated_local(ui: builder::DialogSetup) -> Result<()> {
    ui.dialog().hide();

    if let Some(path) = ui::utils::folder_chooser_dialog(&gettext("Setup Existing Repository"))
        .await
        .and_then(|x| x.path())
    {
        ui.dialog().show();
        if ui::utils::is_backup_repo(&path) {
            let result =
                add_first_try(local::Repository::from_path(path).into_config(), ui.clone()).await;
            // add_first_try moves us to detail, fix here for now
            if !matches!(result, Err(Error::UserCanceled) | Ok(())) {
                eprintln!("{:#?}", result);
                ui.leaflet().set_visible_child(&ui.page_overview());
            }
            return result;
        } else {
            return Err(Message::new(
                gettext("Location is not a valid backup repository."),
                gettext("The repository must originate from Pika Backup or compatible software."),
            )
            .into());
        }
    } else {
        ui.dialog().present();
    }

    Ok(())
}

pub async fn add_button_clicked(ui: builder::DialogSetup) -> Result<()> {
    let remote_location = RemoteLocation::from_user_input(ui.location_url().text().to_string())
        .err_to_msg(gettext("Invalid Remote Location"))?;

    debug!("Add existing URI '{:?}'", remote_location.url());

    let repo = if remote_location.is_borg_host() {
        config::remote::Repository::from_uri(remote_location.url()).into_config()
    } else {
        mount_fuse_and_config(&remote_location.as_gio_file(), false)
            .await?
            .into_config()
    };

    add_first_try(repo, ui).await
}

pub async fn on_init_button_clicked(ui: builder::DialogSetup) -> Result<()> {
    let result = init_repo(ui.clone()).await;

    if result.is_ok() {
        ui.dialog().close();
    } else {
        ui.leaflet().set_visible_child(&ui.page_detail());
    }

    result
}

async fn init_repo(ui: builder::DialogSetup) -> Result<()> {
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
        let remote_location = RemoteLocation::from_user_input(ui.location_url().text().to_string())
            .err_to_msg(gettext("Invalid Remote Location"))?;

        if remote_location.is_borg_host() {
            Ok(config::remote::Repository::from_uri(remote_location.url()).into_config())
        } else {
            mount_fuse_and_config(&remote_location.as_gio_file(), true)
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

    ui.leaflet().set_visible_child(&ui.page_creating());

    let mut borg = borg::CommandOnlyRepo::new(repo.clone());
    let password = config::Password::new(ui.password().text().to_string());
    if encrypted {
        borg.set_password(password.clone());
    }

    let info =
        ui::utils::borg::exec_repo_only(&gettext("Creating Backup Repository"), borg, |borg| {
            borg.init()
        })
        .await
        .into_message("Failed to Initialize Repository")?;

    let config = config::Backup::new(repo.clone(), info, encrypted);

    insert_backup_config(config.clone())?;
    if encrypted {
        ui::utils::secret_service::store_password(&config, &password).await?;
    }
    ui::page_backup::view_backup_conf(&config.id);

    Ok(())
}

pub async fn add_first_try(mut repo: config::Repository, ui: builder::DialogSetup) -> Result<()> {
    repo.set_settings(Some(BackupSettings {
        command_line_args: Some(command_line_args(&ui)?),
    }));

    ui.add_task().set_repo(Some(repo.clone()));

    add(ui).await
}

pub async fn add(ui: builder::DialogSetup) -> Result<()> {
    display::pending_check(&ui);

    let repo = ui.add_task().repo().unwrap();

    let mut borg = borg::CommandOnlyRepo::new(repo.clone());

    if !ui.ask_password().text().is_empty() {
        borg.password = Some(config::Password::new(ui.ask_password().text().to_string()));
    }

    let result =
        ui::utils::borg::exec_repo_only(&gettext("Loading Backup Repository"), borg, |borg| {
            borg.peek()
        })
        .await;

    if matches!(
        result,
        Err(ui::error::Combined::Borg(borg::Error::Failed(
            borg::Failure::PassphraseWrong
        )))
    ) {
        display::ask_password(&ui);

        return Err(Error::UserCanceled);
    }

    if result.is_err() {
        ui.leaflet().set_visible_child(&ui.page_detail());
    }

    let info = result.into_message(gettext("Failed to Configure Repository"))?;

    let encrypted = !ui.ask_password().text().is_empty();

    let config = config::Backup::new(repo.clone(), info, encrypted);
    insert_backup_config(config.clone())?;
    ui::page_backup::view_backup_conf(&config.id);
    ui::utils::secret_service::store_password(
        &config,
        &config::Password::new(ui.ask_password().text().to_string()),
    )
    .await?;

    ui.leaflet().set_visible_child(&ui.page_transfer());

    let archives = ui::utils::borg::exec(borg::Command::<borg::task::List>::new(config.clone()))
        .await
        .into_message(gettext("Failed"))?;

    display::transfer_selection(&ui, config.id.clone(), archives);

    Ok(())
}

fn insert_backup_config(config: config::Backup) -> Result<()> {
    BACKUP_CONFIG.update_result(move |s| {
        s.insert(config.clone())?;
        Ok(())
    })?;

    ui::write_config()
}

pub fn execute<
    F: std::future::Future<Output = Result<()>> + 'static,
    W: IsA<gtk::Window> + IsA<gtk::Widget>,
>(
    f: F,
    window: W,
) {
    Handler::new().error_transient_for(window).spawn(f);
}

fn command_line_args(ui: &builder::DialogSetup) -> Result<Vec<String>> {
    let (start, end) = ui.command_line_args().buffer().bounds();
    if let Ok(args) = shell_words::split(
        &ui.command_line_args()
            .buffer()
            .text(&start, &end, false)
            .to_string(),
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
