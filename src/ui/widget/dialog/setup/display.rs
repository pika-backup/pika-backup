use crate::ui::prelude::*;
use adw::prelude::*;
use adw::subclass::prelude::*;

use itertools::Itertools;
use std::collections::BTreeSet;
use std::fmt::Write;

use crate::borg;
use crate::config;
use crate::ui;

use super::imp;
use super::transfer_option::SetupTransferOption;

#[derive(Clone, Debug)]
pub struct ArchiveParams {
    pub prefix: Option<config::ArchivePrefix>,
    pub parsed: borg::invert_command::Parsed,
    pub hostname: String,
    pub username: String,
    pub end: chrono::NaiveDateTime,
    pub stats: borg::json::Stats,
}

fn extract_archive_params(archive: borg::ListArchive) -> ArchiveParams {
    let prefix = archive
        .name
        .as_str()
        .split_once('-')
        .map(|x| config::ArchivePrefix(x.0.to_string() + "-"));
    let stats = borg::json::Stats::transfer_history_mock(&archive);
    let parsed = borg::invert_command::parse(archive.command_line);

    ArchiveParams {
        prefix,
        parsed,
        hostname: archive.hostname,
        username: archive.username,
        end: archive.end,
        stats,
    }
}

impl imp::SetupDialog {
    pub fn transfer_selection(
        &self,
        config_id: config::ConfigId,
        archives: Vec<borg::ListArchive>,
    ) {
        let archive_params: Vec<_> = archives
            .into_iter()
            .map(extract_archive_params)
            .rev()
            .collect();

        let valid_prefixes: Vec<_> = archive_params
            .iter()
            .map(|x| &x.prefix)
            .duplicates()
            .collect();

        let mut options = archive_params
            .iter()
            .filter(|x| valid_prefixes.contains(&&x.prefix))
            .unique_by(|x| (&x.prefix, &x.parsed, &x.hostname, &x.username))
            .peekable();

        if options.peek().is_none() {
            self.obj().close();
        } else {
            for suggestion in options.take(10) {
                let row = SetupTransferOption::new(suggestion);

                let dialog = self.obj();
                row.transfer_row().connect_activated(
                    clone!(@weak dialog, @strong suggestion, @strong config_id => move |_|
                    Handler::handle(dialog.imp().insert_transfer(&suggestion, &config_id))
                    ),
                );

                self.transfer_suggestions.append(&row);
            }

            self.page_transfer_stack
                .set_visible_child(&*self.page_transfer_select);
        }
    }

    fn insert_transfer(&self, archive_params: &ArchiveParams, config_id: &ConfigId) -> Result<()> {
        BACKUP_CONFIG.try_update(enclose!((archive_params, config_id) move |config| {
        let conf = config.try_get_mut(&config_id)?;

        conf.include = archive_params.parsed.include.clone();
        conf.exclude = BTreeSet::from_iter( archive_params.parsed.exclude.clone().into_iter().map(|x| x.into_relative()));

        Ok(())
    }))?;

        let entry = config::history::RunInfo {
            end: archive_params
                .end
                .and_local_timezone(chrono::Local)
                .unwrap(),
            outcome: borg::Outcome::Completed {
                stats: archive_params.stats.clone(),
            },
            messages: Default::default(),
            include: archive_params.parsed.include.clone(),
            exclude: archive_params.parsed.exclude.clone(),
        };

        BACKUP_HISTORY.try_update(enclose!((config_id) move |histories| {
            histories.insert(config_id.clone(), entry.clone());
            Ok(())
        }))?;

        // Create fake history entry for duration estimate to be good for first run

        main_ui().page_detail().backup_page().refresh()?;

        let configs = BACKUP_CONFIG.load();
        let config = configs.try_get(config_id)?;

        if let Some(prefix) = &archive_params.prefix {
            self.prefix.set_text(&prefix.to_string());
        } else {
            self.prefix.set_text(&config.archive_prefix.to_string());
        }
        self.prefix.grab_focus();
        let dialog = self.obj();

        self.prefix_submit.connect_clicked(clone!(@weak dialog, @strong config_id => move |_|
        Handler::new().error_transient_for(dialog.clone()).handle_sync(dialog.imp().set_prefix(config_id.clone()))));

        self.navigation_view.push(&*self.page_transfer_prefix);

        Ok(())
    }

    pub fn set_prefix(&self, config_id: ConfigId) -> Result<()> {
        let prefix = self.prefix.text();

        BACKUP_CONFIG.try_update(enclose!((prefix, config_id) move |config| {
            config
                .try_get_mut(&config_id)?
                .set_archive_prefix(
                    config::ArchivePrefix::new(&prefix),
                    BACKUP_CONFIG.load().iter(),
                )
                .err_to_msg(gettext("Invalid Archive Prefix"))
        }))?;

        self.obj().close();

        Ok(())
    }

    pub fn show_init_remote(&self) {
        self.location_group_local.set_visible(false);
        self.location_group_remote.set_visible(true);
        self.show_init();
    }

    pub fn show_init_local(&self, path: Option<&std::path::Path>) {
        if let Some(path) = path {
            self.init_path
                .set_property("file", gio::File::for_path(path));
        }

        self.location_group_local.set_visible(true);
        self.location_group_remote.set_visible(false);
        self.show_init();
    }

    fn show_init(&self) {
        self.init_dir.set_text(&format!(
            "backup-{}-{}",
            glib::host_name(),
            glib::user_name().to_string_lossy()
        ));

        self.encryption_preferences_group.reset(true);
        self.navigation_view.push(&*self.page_detail);
        self.ask_password.set_text("");
        self.button_stack
            .set_visible_child(&*self.page_detail_continue);
    }

    pub fn pending_check(&self) {
        self.page_password_stack
            .set_visible_child(&*self.page_password_pending);
        self.page_password.set_can_pop(false);

        if self.navigation_view.visible_page().as_ref() != Some(&*self.page_password) {
            self.navigation_view.push(&*self.page_password);
        }
    }

    pub fn ask_password(&self) {
        self.page_password.set_can_pop(true);
        self.page_password_stack
            .set_visible_child(&*self.page_password_input);
    }

    pub async fn add_mount<F: 'static + Fn()>(
        list: &gtk::ListBox,
        mount: &gio::Mount,
        repo: Option<&std::path::Path>,
        display_fn: F,
    ) {
        let row = ui::utils::new_action_row_with_gicon(Some(mount.icon().as_ref()));
        list.append(&row);

        row.set_widget_name(&mount.root().uri());
        row.connect_activated(move |_| display_fn());
        row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));

        let mut label1 = mount.name().to_string();

        let mut label2: String = mount
            .drive()
            .as_ref()
            .map(gio::Drive::name)
            .map(Into::into)
            .unwrap_or_else(|| mount.root().uri().to_string());

        if let Some(mount_path) = mount.root().path() {
            if let Ok(df) = ui::utils::df::local(&mount_path).await {
                let _ = write!(label1, " – {}", &glib::format_size(df.size));

                label2.push_str(" – ");
                label2.push_str(&gettextf("Free space: {}", &[&glib::format_size(df.avail)]));
            }

            if let Some(repo_path) = repo {
                row.set_widget_name(&gio::File::for_path(repo_path).uri());
                if let Ok(suffix) = repo_path.strip_prefix(mount_path) {
                    if !suffix.to_string_lossy().is_empty() {
                        let _ = write!(label1, " / {}", suffix.display());
                    }
                }
            }
        }

        row.set_title(&glib::markup_escape_text(&label1));
        row.set_subtitle(&glib::markup_escape_text(&label2));
    }
}
