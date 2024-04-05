use crate::{borg, config, ui::prelude::*};
use adw::subclass::prelude::*;

#[derive(Clone, Debug, glib::Boxed)]
#[boxed_type(name = "PkSetupArchiveParams", nullable)]
pub struct ArchiveParams {
    pub prefix: Option<config::ArchivePrefix>,
    pub parsed: borg::invert_command::Parsed,
    pub hostname: String,
    pub username: String,
    pub end: chrono::NaiveDateTime,
    pub stats: borg::json::Stats,
}

impl From<borg::ListArchive> for ArchiveParams {
    fn from(archive: borg::ListArchive) -> Self {
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
}

mod imp {

    use crate::ui::{self, widget::WrapBox};

    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(file = "transfer_option.ui")]
    pub struct SetupTransferOption {
        #[template_child]
        pub(super) transfer_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        hostname: TemplateChild<gtk::Label>,
        #[template_child]
        username: TemplateChild<gtk::Label>,
        #[template_child]
        prefix: TemplateChild<gtk::Label>,
        #[template_child]
        include_box: TemplateChild<WrapBox>,
        #[template_child]
        exclude_box: TemplateChild<WrapBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SetupTransferOption {
        const NAME: &'static str = "PkSetupTransferOption";
        type Type = super::SetupTransferOption;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SetupTransferOption {}
    impl WidgetImpl for SetupTransferOption {}
    impl ListBoxRowImpl for SetupTransferOption {}

    #[gtk::template_callbacks]
    impl SetupTransferOption {
        pub(super) fn set_suggestion(&self, suggestion: &ArchiveParams) {
            self.hostname.set_label(&suggestion.hostname);
            self.username.set_label(&suggestion.username);
            self.prefix.set_label(
                &suggestion
                    .prefix
                    .as_ref()
                    .map(|x| x.to_string())
                    .unwrap_or_else(|| gettext("None")),
            );

            for include in suggestion.parsed.include.iter() {
                let tag = ui::widget::LocationTag::from_path(include.clone());
                self.include_box.add_child(&tag.build());
            }

            for exclude in suggestion.parsed.exclude.iter() {
                let tag = ui::widget::LocationTag::from_exclude(exclude.clone().into_relative());
                self.exclude_box.add_child(&tag.build());
            }
        }
    }
}

glib::wrapper! {
    pub struct SetupTransferOption(ObjectSubclass<imp::SetupTransferOption>)
    @extends gtk::ListBoxRow, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SetupTransferOption {
    pub fn new(suggestion: &ArchiveParams) -> Self {
        let dialog: Self = glib::Object::new();
        dialog.imp().set_suggestion(suggestion);
        dialog
    }

    pub fn transfer_row(&self) -> adw::ActionRow {
        self.imp().transfer_row.clone()
    }
}
