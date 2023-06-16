use crate::config::*;
use adw::prelude::*;

use super::display;
use super::insert;
use super::insert::execute;
use crate::ui::*;
use ui::builder::DialogSetup;

pub fn navigation_view_changed(ui: &DialogSetup) {
    if let Some(visible_page) = ui.navigation_view().visible_page() {
        if visible_page == ui.page_overview() {
            ui.init_path().reset();
            ui.location_url().set_text("");
            ui.password().set_text("");
            ui.password_confirm().set_text("");
        }

        if visible_page == ui.page_overview() || visible_page == ui.page_detail() {
            ui.ask_password().set_text("");
        }
    }

    if ui.add_button().is_mapped() {
        ui.dialog().set_default_widget(Some(&ui.add_button()));
    }

    if ui.init_button().is_mapped() {
        ui.dialog().set_default_widget(Some(&ui.init_button()));
    }

    if ui.page_password_continue().is_mapped() {
        ui.dialog()
            .set_default_widget(Some(&ui.page_password_continue()));
    }

    if ui.location_url().is_mapped() {
        ui.location_url().grab_focus();
    }

    if ui.init_dir().is_mapped() {
        if ui.init_path().file().is_none() {
            ui.init_path().grab_focus();
        } else {
            ui.init_dir().grab_focus();
        }
    }

    if ui.ask_password().is_mapped() {
        ui.ask_password().grab_focus();
    }
}

pub fn page_detail_continue(ui: &DialogSetup) {
    execute(super::insert::validate_detail_page(ui.clone()), ui.dialog());
}

pub fn show_init_local(ui: &DialogSetup) {
    display::show_init_local(ui, None);
}

pub fn show_init_remote(ui: &DialogSetup) {
    display::show_init_remote(ui);
}

pub fn init_repo(ui: &DialogSetup) {
    execute(insert::on_init_button_clicked(ui.clone()), ui.dialog());
}

pub fn show_add_local(ui: &DialogSetup) {
    execute(
        insert::on_add_repo_list_activated_local(ui.clone()),
        ui.dialog(),
    );
}

pub async fn page_password_continue(ui: DialogSetup) -> Result<()> {
    insert::add(ui).await
}

pub fn show_add_remote(ui: &DialogSetup) {
    ui.button_stack().set_visible_child(&ui.add_button());
    ui.encryption_box().hide();
    ui.location_group_local().set_visible(false);
    ui.location_group_remote().set_visible(true);
    ui.navigation_view().push(&ui.page_detail());
}

pub fn add_local(ui: &DialogSetup, path: Option<&std::path::Path>) {
    if let Some(path) = path {
        execute(
            insert::add_first_try(
                local::Repository::from_path(path.to_path_buf()).into_config(),
                ui.clone(),
            ),
            ui.dialog(),
        );
    }
}

pub async fn add_remote(ui: DialogSetup) -> Result<()> {
    insert::add_button_clicked(ui).await
}

pub fn path_change(ui: &DialogSetup) {
    if let Some(path) = ui.init_path().file().and_then(|x| x.path()) {
        let mount_entry = gio::UnixMountEntry::for_file_path(path);
        if let Some(fs) = mount_entry.0.map(|x| x.fs_type()) {
            debug!("Selected filesystem type {}", fs);
            ui.non_journaling_warning()
                .set_visible(crate::NON_JOURNALING_FILESYSTEMS.iter().any(|x| x == &fs));
        } else {
            ui.non_journaling_warning().hide();
        }
    } else {
        ui.non_journaling_warning().hide();
    }
}

pub fn password_changed(ui: &DialogSetup) {
    let password = ui.password().text();
    let score = if let Ok(pw_check) = zxcvbn::zxcvbn(&password, &[]) {
        if pw_check.score() > 3 {
            let n = pw_check.guesses_log10();
            if (12.0..13.0).contains(&n) {
                5
            } else if (13.0..14.0).contains(&n) {
                6
            } else if n > 14.0 {
                7
            } else {
                4
            }
        } else {
            pw_check.score()
        }
    } else {
        0
    };

    ui.password_quality().set_value(score.into());

    // Show warning highlight if passwords don't match
    if !ui.password_confirm().text().is_empty() {
        if ui.password().text() == ui.password_confirm().text() {
            ui.password_confirm().add_css_class("success");
            ui.password_confirm().remove_css_class("warning");
        } else {
            ui.password_confirm().remove_css_class("success");
            ui.password_confirm().add_css_class("warning");
        }
    } else {
        ui.password_confirm().remove_css_class("success");
        ui.password_confirm().remove_css_class("warning");
    }
}
