use chrono::prelude::*;
use gtk::prelude::*;
use libhandy::prelude::*;

use crate::shared;
use crate::ui;
use crate::ui::globals::*;
use crate::ui::prelude::*;
use crate::ui::utils::WidgetEnh;

pub fn init() {
    if SETTINGS.load().backups.len() > 1 {
        main_ui()
            .main_stack()
            .set_visible_child(&main_ui().page_overview());
        refresh();
    } else if let Some(ref config) = SETTINGS.load().backups.values().next() {
        ui::page_detail::view_backup_conf(&config.id);
    } else {
        main_ui()
            .main_stack()
            .set_visible_child(&main_ui().page_overview_empty());
    }

    main_ui()
        .main_stack()
        .connect_property_visible_child_notify(on_main_stack_changed);

    main_ui()
        .add_backup()
        .connect_clicked(|_| ui::dialog_add_config::new_backup());
    main_ui()
        .add_backup_empty()
        .connect_clicked(|_| ui::dialog_add_config::new_backup());

    main_ui().remove_backup().connect_clicked(on_remove_backup);

    main_ui()
        .main_stack()
        .connect_property_transition_running_notify(on_transition);
}

fn on_transition(stack: &gtk::Stack) {
    if (!stack.get_transition_running())
        && stack.get_visible_child() != Some(main_ui().page_overview().upcast::<gtk::Widget>())
    {
        // get rid of potential GtkSpinner's for performance reasons
        ui::utils::clear(&main_ui().main_backups());
    }
}

fn on_main_stack_changed(stack: &gtk::Stack) {
    if stack.get_visible_child() == Some(main_ui().page_overview().upcast::<gtk::Widget>()) {
        refresh();
    }
}

fn on_remove_backup(_button: &gtk::ModelButton) {
    let delete = ui::utils::dialog_yes_no(gettext(
        "Are you sure you want to delete this backup configuration?",
    ));

    if delete {
        if let Some(config) = SETTINGS.get().backups.get_active() {
            SETTINGS.update(move |s| {
                s.backups.remove(&config.id);
            });

            ui::utils::dialog_catch_err(
                ui::utils::secret_service_delete_passwords(config),
                "Failed to remove potentially remaining passwords from key storage.",
            );

            ACTIVE_BACKUP_ID.update(|active_id| *active_id = None);
            ui::write_config();

            if SETTINGS.load().backups.is_empty() {
                main_ui()
                    .main_stack()
                    .set_visible_child(&main_ui().page_overview_empty());
            } else {
                main_ui()
                    .main_stack()
                    .set_visible_child(&main_ui().page_overview());
            };
        } else {
            ui::utils::dialog_error(gettext("No active backup to delete."));
        }
    }
}

fn refresh() {
    let list = main_ui().main_backups();
    ui::utils::clear(&list);

    for config in SETTINGS.load().backups.values() {
        let row = libhandy::ActionRow::new();
        list.add(&row);

        row.set_activatable(true);
        row.connect_activated(enclose!((config) move |_| {
            ui::page_detail::view_backup_conf(&config.id);
        }));

        let main_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
        row.add_prefix(&main_box);

        // Repo Icon

        let repo_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        repo_box.add_css_class("backup-repo");
        main_box.add(&repo_box);

        if let Ok(icon) = gio::Icon::new_for_string(&ui::utils::repo_icon(&config.repo)) {
            repo_box.add(&gtk::Image::from_gicon(&icon, gtk::IconSize::Button));
        }

        // Repo Name

        let mut location = String::new();

        if let shared::BackupRepo::Local {
            label,
            device: Some(device),
            ..
        } = &config.repo
        {
            location = format!(
                "{} – {}",
                label.as_ref().map(|x| x.as_str()).unwrap_or_default(),
                device,
            )
        } else {
            location.push_str(&config.repo.to_string());
        }

        let label = gtk::Label::new(Some(&location));
        label.set_ellipsize(pango::EllipsizeMode::Middle);
        repo_box.add(&label);

        // Include

        for path in &config.include {
            let incl = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            incl.add_css_class("backup-include");
            main_box.add(&incl);

            if let Some(icon) =
                ui::utils::file_symbolic_icon(&shared::absolute(path), gtk::IconSize::Button)
            {
                incl.add(&icon);
            }

            let path_str = if path.iter().next().is_none() {
                gettext("Home")
            } else {
                path.to_string_lossy().to_string()
            };

            let label = gtk::Label::new(Some(&path_str));
            label.set_ellipsize(pango::EllipsizeMode::Middle);
            incl.add(&label);
        }

        // Status

        let status_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        status_box.add_css_class("backup-status");
        main_box.add(&status_box);
        if BACKUP_COMMUNICATION.load().contains_key(&config.id) {
            let spinner = gtk::Spinner::new();
            spinner.start();
            status_box.add(&spinner);
            status_box.add(&gtk::Label::new(Some(&gettext("Backup running"))));
        } else {
            match config.last_run {
                Some(shared::RunInfo {
                    end, result: Ok(_), ..
                }) => {
                    status_box.add(&gtk::Image::from_icon_name(
                        Some("emblem-default-symbolic"),
                        gtk::IconSize::Button,
                    ));
                    status_box.add(&gtk::Label::new(Some(&gettextf(
                        "Last backup finished about {}",
                        &[&(end - Local::now()).humanize()],
                    ))));
                }
                None => status_box.add(&gtk::Label::new(Some(&gettext("Backup never ran")))),
                _ => {
                    status_box.add_css_class("backup-failed");
                    status_box.add(&gtk::Image::from_icon_name(
                        Some("dialog-error-symbolic"),
                        gtk::IconSize::Button,
                    ));
                    status_box.add(&gtk::Label::new(Some(&gettext("Last backup failed"))));
                }
            }
        }
    }
    list.show_all();
}
