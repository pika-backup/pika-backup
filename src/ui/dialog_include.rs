use adw::prelude::*;
use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::ui;
use ui::builder::DialogInclude;
use ui::config;
use ui::prelude::*;

const SUGGESTED_USER_DIRS: &[glib::UserDirectory] = &[
    glib::UserDirectory::Documents,
    glib::UserDirectory::Downloads,
    glib::UserDirectory::Music,
    glib::UserDirectory::Pictures,
    glib::UserDirectory::PublicShare,
    glib::UserDirectory::Videos,
];

pub fn show() {
    let ui = DialogInclude::new();
    ui.dialog().set_transient_for(Some(&main_ui().window()));

    ui.exclude_folder()
        .connect_activated(glib::clone!(@weak ui => move |_| {
            ui.dialog().destroy();
            Handler::run(include_folder())
        }));

    ui.exclude_file()
        .connect_activated(glib::clone!(@weak ui => move |_| {
            ui.dialog().destroy();
            Handler::run(include_file())
        }));

    // ensure lifetime until window closes
    let mutex = std::sync::Mutex::new(Some(ui.clone()));
    ui.dialog().connect_close_request(move |_| {
        *mutex.lock().unwrap() = None;
        gtk::Inhibit(false)
    });

    Handler::handle(fill_suggestions(&ui));

    ui.dialog().show();
}

pub fn fill_suggestions(dialog: &DialogInclude) -> Result<()> {
    let mut predefined = vec![(gettext("Home"), PathBuf::new())];

    for dir_type in SUGGESTED_USER_DIRS {
        if let Some(path) = glib::user_special_dir(*dir_type) {
            if path != glib::home_dir() {
                predefined.push((
                    config::path_relative(&path).to_string_lossy().to_string(),
                    config::path_relative(path),
                ));
            }
        }
    }

    let mut buttons = Vec::new();
    let configs = &BACKUP_CONFIG.load();
    let include = &configs.active().unwrap().include;

    for (name, path) in predefined {
        let check_button = gtk::CheckButton::new();
        if include.contains(&path) {
            check_button.set_active(true);
        }

        let row = adw::ActionRow::builder()
            .title(&name)
            .activatable_widget(&check_button)
            .build();

        row.add_prefix(&check_button);

        dialog.suggestions().add(&row);
        buttons.push((path, check_button));
    }

    let buttons = Rc::new(buttons);

    for (_, button) in buttons.iter() {
        // TODO: potential memory leak
        let buttons = buttons.clone();
        button.connect_toggled(move |_| Handler::handle(on_suggested_toggle(&buttons)));
    }

    Ok(())
}

fn on_suggested_toggle(buttons: &[(PathBuf, gtk::CheckButton)]) -> Result<()> {
    let predefined = buttons
        .iter()
        .map(|(path, button)| (path, button.is_active()));

    // TODO: store config id in dialog
    let new_include: BTreeSet<PathBuf> = BACKUP_CONFIG
        .load()
        .active()?
        .include
        .clone()
        .into_iter()
        .filter(|x| !predefined.clone().any(|y| y.0 == x))
        .chain(
            predefined
                .clone()
                .filter(|(_, active)| *active)
                .map(|(path, _)| path.to_owned()),
        )
        .collect();

    BACKUP_CONFIG.update_result(move |settings| {
        settings.active_mut()?.include = new_include.clone();

        Ok(())
    })?;

    for (path, button) in buttons {
        if buttons[0].1.is_active() && path.is_relative() && !button.is_active() {
            button.set_inconsistent(true);
        } else {
            button.set_inconsistent(false);
        }
    }

    crate::ui::write_config()?;
    ui::page_backup::refresh()?;

    Ok(())
}

pub async fn include_folder() -> Result<()> {
    let chooser = gtk::FileChooserNative::builder()
        .action(gtk::FileChooserAction::SelectFolder)
        .select_multiple(true)
        .title(&gettext("Include Directory"))
        .accept_label(&gettext("Select"))
        .modal(true)
        .transient_for(&main_ui().window())
        .build();

    let paths = ui::utils::paths(chooser).await?;

    BACKUP_CONFIG.update_result(|settings| {
        for path in &paths {
            settings
                .active_mut()?
                .include
                .insert(ui::utils::rel_path(path));
        }
        Ok(())
    })?;
    crate::ui::write_config()?;
    ui::page_backup::refresh()?;

    Ok(())
}

pub async fn include_file() -> Result<()> {
    let chooser = gtk::FileChooserNative::builder()
        .action(gtk::FileChooserAction::Open)
        .select_multiple(true)
        .title(&gettext("Include File"))
        .accept_label(&gettext("Select"))
        .modal(true)
        .transient_for(&main_ui().window())
        .build();

    let paths = ui::utils::paths(chooser).await?;

    BACKUP_CONFIG.update_result(|settings| {
        for path in &paths {
            settings
                .active_mut()?
                .include
                .insert(ui::utils::rel_path(path));
        }
        Ok(())
    })?;

    crate::ui::write_config()?;
    ui::page_backup::refresh()?;

    Ok(())
}
