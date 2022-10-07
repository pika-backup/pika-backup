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
        gtk::Inhibit(false)
    });

    Handler::handle(fill_suggestions(&ui));

    ui.dialog().show();
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

        let popover = gtk::Popover::builder()
            .child(&gtk::Label::new(Some(&format!(
                "{:#?}",
                predefined.borg_rules()
            ))))
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

    BACKUP_CONFIG.update_result(move |settings| {
        settings.active_mut()?.exclude = new_exclude.clone();

        Ok(())
    })?;

    crate::ui::write_config()?;
    ui::page_backup::refresh()?;

    Ok(())
}

pub async fn exclude_folder() -> Result<()> {
    let chooser = gtk::FileChooserNative::builder()
        .action(gtk::FileChooserAction::SelectFolder)
        .select_multiple(true)
        .title(&gettext("Exclude Directory"))
        .accept_label(&gettext("Select"))
        .modal(true)
        .transient_for(&main_ui().window())
        .build();

    let paths = ui::utils::paths(chooser).await?;

    BACKUP_CONFIG.update_result(|settings| {
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
    crate::ui::write_config()?;
    ui::page_backup::refresh()?;

    Ok(())
}

pub async fn exclude_file() -> Result<()> {
    let chooser = gtk::FileChooserNative::builder()
        .action(gtk::FileChooserAction::Open)
        .select_multiple(true)
        .title(&gettext("Exclude File"))
        .accept_label(&gettext("Select"))
        .modal(true)
        .transient_for(&main_ui().window())
        .build();

    let paths = ui::utils::paths(chooser).await?;

    BACKUP_CONFIG.update_result(|settings| {
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

    crate::ui::write_config()?;
    ui::page_backup::refresh()?;

    Ok(())
}

pub async fn exclude_pattern() -> Result<()> {
    ui::dialog_exclude_pattern::show();
    Ok(())
}
