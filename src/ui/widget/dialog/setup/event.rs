use crate::config::*;
use adw::prelude::*;
use adw::subclass::prelude::*;

use super::imp;
use crate::ui::*;

impl imp::SetupDialog {
    pub fn event_navigation_view_changed(&self) {
        if let Some(visible_page) = self.navigation_view.visible_page() {
            if visible_page == *self.start_page {
                self.init_path.reset();
                self.location_url.set_text("");
                self.encryption_preferences_group.reset(true);
            }

            if visible_page == *self.start_page || visible_page == *self.page_detail {
                self.ask_password.set_text("");
            }
        }

        if self.location_url.is_mapped() {
            self.location_url.grab_focus();
        }

        if self.init_dir.is_mapped() {
            if self.init_path.file().is_none() {
                self.init_path.grab_focus();
            } else {
                self.init_dir.grab_focus();
            }
        }

        if self.ask_password.is_mapped() {
            self.ask_password.grab_focus();
        }
    }

    pub fn event_page_detail_continue(&self) {
        let obj = self.obj().clone();
        Self::execute(
            async move { obj.imp().validate_detail_page().await },
            self.obj().clone(),
        );
    }

    pub fn event_show_init_local(&self) {
        self.show_init_local(None);
    }

    pub fn event_show_init_remote(&self) {
        self.show_init_remote();
    }

    pub fn event_init_repo(&self) {
        let obj = self.obj().clone();
        Self::execute(
            async move { obj.imp().on_init_button_clicked().await },
            self.obj().clone(),
        );
    }

    pub async fn event_page_password_continue(&self) -> Result<()> {
        self.add().await
    }

    pub fn event_show_add_remote(&self) {
        self.button_stack.set_visible_child(&*self.add_button);
        self.location_group_local.set_visible(false);
        self.location_group_remote.set_visible(true);
        self.navigation_view.push(&*self.page_detail);
    }

    pub fn event_add_local(&self, path: Option<&std::path::Path>) {
        if let Some(path) = path {
            let obj = self.obj().clone();
            let path = path.to_path_buf();
            Self::execute(
                async move {
                    obj.imp()
                        .add_first_try(local::Repository::from_path(path).into_config())
                        .await
                },
                self.obj().clone(),
            );
        }
    }

    pub async fn event_add_remote(&self) -> Result<()> {
        self.add_button_clicked().await
    }

    pub fn event_path_change(&self) {
        if let Some(path) = self.init_path.file().and_then(|x| x.path()) {
            let mount_entry = gio::UnixMountEntry::for_file_path(path);
            if let Some(fs) = mount_entry.0.map(|x| x.fs_type()) {
                debug!("Selected filesystem type {}", fs);
                self.non_journaling_warning
                    .set_visible(crate::NON_JOURNALING_FILESYSTEMS.iter().any(|x| x == &fs));
            } else {
                self.non_journaling_warning.set_visible(false);
            }
        } else {
            self.non_journaling_warning.set_visible(false);
        }
    }
}
