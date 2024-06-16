use num_format::ToFormattedString;

use crate::borg;
use crate::config::history::*;
use crate::ui::backup_status;
use crate::ui::prelude::*;

use adw::prelude::*;
use adw::subclass::prelude::*;

mod imp {
    use crate::ui::widget::StatusRow;

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(file = "backup_info.ui")]
    pub struct BackupInfoDialog {
        #[template_child]
        info_status: TemplateChild<StatusRow>,
        #[template_child]
        info_progress: TemplateChild<gtk::ProgressBar>,

        #[template_child]
        stats: TemplateChild<gtk::ListBox>,
        #[template_child]
        original_size: TemplateChild<gtk::Label>,
        #[template_child]
        nfiles: TemplateChild<gtk::Label>,
        #[template_child]
        deduplicated_size: TemplateChild<gtk::Label>,
        #[template_child]
        path_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        current_path: TemplateChild<gtk::Label>,

        #[template_child]
        info_error: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BackupInfoDialog {
        const NAME: &'static str = "PkBackupInfoDialog";
        type Type = super::BackupInfoDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for BackupInfoDialog {}
    impl WidgetImpl for BackupInfoDialog {}
    impl AdwDialogImpl for BackupInfoDialog {}

    #[gtk::template_callbacks]
    impl BackupInfoDialog {
        pub(super) fn refresh_status_display(&self, status: &backup_status::Display) {
            self.info_status.set_from_backup_status(status);

            if let Some(progress) = status.progress {
                self.info_progress.set_fraction(progress);
                self.info_progress.set_visible(true);
            } else {
                self.info_progress.set_visible(false);
            }

            if let Some(backup_status::Stats::Final(run_info)) = &status.stats {
                let mut message = String::new();

                if !matches!(run_info.outcome, borg::Outcome::Completed { .. }) {
                    message.push_str(&run_info.outcome.to_string());
                    message.push_str("\n\n");
                }

                message.push_str(&run_info.messages.clone().filter_hidden().to_string());

                self.info_error.set_text(&message);
                self.info_error.set_visible(true);
            } else {
                self.info_error.set_visible(false);
            }

            match &status.stats {
                Some(backup_status::Stats::Final(RunInfo {
                    outcome: borg::Outcome::Completed { stats },
                    ..
                })) => {
                    self.stats.set_visible(true);
                    self.path_row.set_visible(false);

                    self.original_size
                        .set_text(&glib::format_size(stats.archive.stats.original_size));
                    self.deduplicated_size
                        .set_text(&glib::format_size(stats.archive.stats.deduplicated_size));
                    self.nfiles
                        .set_text(&stats.archive.stats.nfiles.to_formatted_string(&*LC_LOCALE));
                }
                Some(backup_status::Stats::Progress(progress_archive)) => {
                    self.stats.set_visible(true);
                    self.path_row.set_visible(true);

                    self.original_size
                        .set_text(&glib::format_size(progress_archive.original_size));
                    self.deduplicated_size
                        .set_text(&glib::format_size(progress_archive.deduplicated_size));
                    self.nfiles
                        .set_text(&progress_archive.nfiles.to_formatted_string(&*LC_LOCALE));

                    self.current_path
                        .set_text(&format!("/{}", progress_archive.path));
                    self.current_path
                        .set_tooltip_text(Some(&format!("/{}", progress_archive.path)));
                }
                _ => {
                    self.stats.set_visible(false);
                }
            }
        }
    }
}

glib::wrapper! {
    pub struct BackupInfoDialog(ObjectSubclass<imp::BackupInfoDialog>)
    @extends gtk::Widget, adw::Dialog,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl BackupInfoDialog {
    pub fn refresh_status_display(&self, status: &backup_status::Display) {
        self.imp().refresh_status_display(status);
    }
}
