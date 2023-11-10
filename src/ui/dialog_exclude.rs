use adw::prelude::*;

use crate::ui;
use ui::builder::DialogExclude;
use ui::config;
use ui::prelude::*;

use std::collections::BTreeSet;
use std::rc::Rc;

//config: &config::Backup
pub fn show() {
    let ui = DialogExclude::new();
    ui.dialog().set_transient_for(Some(&main_ui().window()));

    ui.exclude_folder()
        .connect_activated(glib::clone!(@weak ui => move |_| {
            ui.dialog().destroy();
            Handler::run(exclude_folder())
        }));

    ui.exclude_file()
        .connect_activated(glib::clone!(@weak ui => move |_| {
            ui.dialog().destroy();
            Handler::run(exclude_file())
        }));

    ui.exclude_pattern()
        .connect_activated(glib::clone!(@weak ui => move |_| {
            ui.dialog().destroy();
            Handler::run(exclude_pattern())
        }));

    // ensure lifetime until window closes
    let mutex = std::sync::Mutex::new(Some(ui.clone()));
    ui.dialog().connect_close_request(move |_| {
        *mutex.lock().unwrap() = None;
        glib::Propagation::Proceed
    });

    Handler::handle(fill_suggestions(&ui));
    Handler::handle(fill_unreadable(&ui));

    ui.dialog().present();
}

pub fn fill_suggestions(dialog: &DialogExclude) -> Result<()> {
    let mut buttons = Vec::new();
    let configs = &BACKUP_CONFIG.load();
    let exclude = &configs.active().unwrap().exclude;

    for predefined in config::exclude::Predefined::VALUES {
        let check_button = gtk::CheckButton::new();
        if exclude.contains(&config::Exclude::from_predefined(predefined.clone())) {
            check_button.set_active(true);
        }

        let row = adw::ActionRow::builder()
            .title(&predefined.description())
            .subtitle(&predefined.kind())
            .activatable_widget(&check_button)
            .build();

        row.add_prefix(&check_button);

        let desc = predefined
            .borg_rules()
            .into_iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        let popover = gtk::Popover::builder()
            .child(
                &gtk::Label::builder()
                    .label(&format!("{}\n\n{desc}", gettext("Exclusion Rules")))
                    .selectable(true)
                    .focusable(false)
                    .build(),
            )
            .build();

        let info_button = gtk::MenuButton::builder()
            .icon_name("dialog-information-symbolic")
            .popover(&popover)
            .valign(gtk::Align::Center)
            .build();
        info_button.add_css_class("flat");

        row.add_suffix(&info_button);

        dialog.suggestions().add(&row);
        buttons.push((predefined, check_button));
    }

    let buttons = Rc::new(buttons);

    for (_, button) in buttons.iter() {
        // TODO: potential memory leak
        let buttons = buttons.clone();
        button.connect_toggled(move |_| Handler::handle(on_suggested_toggle(&buttons)));
    }

    Ok(())
}

fn on_suggested_toggle(buttons: &[(config::exclude::Predefined, gtk::CheckButton)]) -> Result<()> {
    let new_predefined = buttons
        .iter()
        .filter(|(_, button)| button.is_active())
        .map(|(predefined, _)| config::Exclude::from_predefined(predefined.clone()));

    // TODO: store config id in dialog
    let new_exclude: BTreeSet<config::Exclude<{ config::RELATIVE }>> = BACKUP_CONFIG
        .load()
        .active()?
        .exclude
        .clone()
        .into_iter()
        .filter(|x| !x.is_predefined())
        .chain(new_predefined)
        .collect();

    BACKUP_CONFIG.try_update(move |settings| {
        settings.active_mut()?.exclude = new_exclude.clone();
        Ok(())
    })?;

    ui::page_backup::refresh()?;

    Ok(())
}

pub fn fill_unreadable(dialog: &DialogExclude) -> Result<()> {
    dialog.unreadable_paths().set_visible(false);

    let configs = BACKUP_CONFIG.load();
    let exclude = &configs.active()?.exclude;

    let histories = BACKUP_HISTORY.load();
    // If the history is missing we don't have any suggested excludes and shouldn't fail
    let suggested_excludes = histories.active().ok().and_then(|history| {
        history
            .suggested_exclude
            .get(&config::history::SuggestedExcludeReason::PermissionDenied)
    });

    let Some(suggested_excludes) = suggested_excludes else {
        return Ok(());
    };

    for suggested in suggested_excludes {
        // We have at least one entry
        dialog.unreadable_paths().set_visible(true);

        let add_button = gtk::CheckButton::builder()
            .tooltip_text(&gettext("Add exclusion rule"))
            .valign(gtk::Align::Center)
            .active(exclude.contains(suggested))
            .build();

        let row = adw::ActionRow::builder()
            .title(suggested.description())
            .activatable_widget(&add_button)
            .build();

        row.add_prefix(&add_button);

        dialog.unreadable_paths().add(&row);

        add_button.connect_toggled(
            glib::clone!(@strong suggested_excludes, @strong suggested, @weak row, @weak dialog => move |button| {
                Handler::handle((|| {
                    BACKUP_CONFIG.try_update(glib::clone!(@strong suggested_excludes, @strong suggested, @weak button => @default-return Ok(()), move |settings| {
                        let active = settings.active_mut()?;

                        if button.is_active() {
                            active.exclude.insert(suggested.clone());
                        } else {
                            active.exclude.remove(&suggested.clone());
                        }

                        Ok(())
                    }))?;

                    ui::page_backup::refresh()?;
                    Ok(())
                })());
            }),
        );
    }

    Ok(())
}

/// Find the common ancestor of all included folders
async fn exclude_base_folder() -> Result<gio::File> {
    let includes = BACKUP_CONFIG.load().active()?.include_dirs();

    // Find the common ancestor
    let mut base: Option<std::path::PathBuf> = None;
    for path in includes {
        if let Some(base_path) = &base {
            for ancestor in path.ancestors() {
                if base_path.starts_with(ancestor) {
                    base = Some(ancestor.to_path_buf());
                    break;
                }
            }
        } else {
            base = Some(path);
        }
    }

    // Make sure this is a directory, not a file
    if let Some(base_path) = &base {
        if async_std::fs::metadata(base_path)
            .await
            .is_ok_and(|meta| meta.is_file())
        {
            base = base_path.parent().map(|p| p.to_path_buf())
        }
    }

    Ok(gio::File::for_path(base.unwrap_or_else(glib::home_dir)))
}

pub async fn exclude_folder() -> Result<()> {
    let chooser = gtk::FileDialog::builder()
        .initial_folder(&exclude_base_folder().await?)
        .title(gettext("Exclude Directory"))
        .accept_label(gettext("Select"))
        .modal(true)
        .build();

    let paths = ui::utils::paths_from_model(
        chooser
            .select_multiple_folders_future(Some(&main_ui().window()))
            .await
            .map_err(|err| match err.kind::<gtk::DialogError>() {
                Some(gtk::DialogError::Cancelled | gtk::DialogError::Dismissed) => {
                    Error::UserCanceled
                }
                _ => Message::short(err.to_string()).into(),
            })?,
    )?;

    BACKUP_CONFIG.try_update(|settings| {
        for path in &paths {
            settings
                .active_mut()?
                .exclude
                .insert(config::Exclude::from_pattern(config::Pattern::path_prefix(
                    path,
                )));
        }
        Ok(())
    })?;

    ui::page_backup::refresh()?;
    Ok(())
}

pub async fn exclude_file() -> Result<()> {
    let chooser = gtk::FileDialog::builder()
        .initial_folder(&exclude_base_folder().await?)
        .title(gettext("Exclude File"))
        .accept_label(gettext("Select"))
        .modal(true)
        .build();

    let paths = ui::utils::paths_from_model(Some(
        chooser
            .open_multiple_future(Some(&main_ui().window()))
            .await
            .map_err(|err| match err.kind::<gtk::DialogError>() {
                Some(gtk::DialogError::Cancelled | gtk::DialogError::Dismissed) => {
                    Error::UserCanceled
                }
                _ => Message::short(err.to_string()).into(),
            })?,
    ))?;

    BACKUP_CONFIG.try_update(|settings| {
        for path in &paths {
            settings
                .active_mut()?
                .exclude
                .insert(config::Exclude::from_pattern(
                    config::Pattern::path_full_match(path),
                ));
        }
        Ok(())
    })?;

    ui::page_backup::refresh()?;
    Ok(())
}

pub async fn exclude_pattern() -> Result<()> {
    ui::dialog_exclude_pattern::show(None);
    Ok(())
}
