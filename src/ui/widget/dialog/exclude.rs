use crate::ui;
use ui::config;
use ui::prelude::*;

use std::collections::BTreeSet;

use adw::prelude::*;
use adw::subclass::prelude::*;

mod imp {
    use std::cell::{OnceCell, RefCell};

    use self::ui::{error::HandleError, App};

    use super::*;

    #[derive(Default, glib::Properties, gtk::CompositeTemplate)]
    #[template(file = "exclude.ui")]
    #[properties(wrapper_type = super::ExcludeDialog)]
    pub struct ExcludeDialog {
        #[property(get, set, construct_only)]
        config: OnceCell<crate::config::Backup>,
        edit_exclude: RefCell<Option<config::Exclude<{ config::RELATIVE }>>>,

        // Navigation
        #[template_child]
        navigation_view: TemplateChild<adw::NavigationView>,

        // Main page
        #[template_child]
        root_page: TemplateChild<adw::NavigationPage>,
        #[template_child]
        suggestions: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        unreadable_paths: TemplateChild<adw::PreferencesGroup>,

        // Pattern page
        #[template_child]
        pattern_page: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pattern_add_button: TemplateChild<gtk::Button>,
        #[template_child]
        pattern_type: TemplateChild<adw::ComboRow>,
        #[template_child]
        pattern: TemplateChild<adw::EntryRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ExcludeDialog {
        const NAME: &'static str = "PkExcludeDialog";
        type Type = super::ExcludeDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ExcludeDialog {
        fn constructed(&self) {
            self.parent_constructed();
            let config = self
                .config
                .get()
                .expect("construct_only property must be set");

            self.fill_suggestions(config);
            self.fill_unreadable(config);
        }
    }
    impl WidgetImpl for ExcludeDialog {}
    impl AdwDialogImpl for ExcludeDialog {}

    #[gtk::template_callbacks]
    impl ExcludeDialog {
        fn fill_suggestions(&self, config: &crate::config::Backup) {
            let exclude = &config.exclude;

            for predefined in config::exclude::Predefined::VALUES {
                let check_button = gtk::CheckButton::new();
                if exclude.contains(&config::Exclude::from_predefined(predefined.clone())) {
                    check_button.set_active(true);
                }

                let row = adw::ActionRow::builder()
                    .title(predefined.description())
                    .subtitle(predefined.kind())
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

                self.suggestions.add(&row);

                check_button.connect_toggled(glib::clone!(
                    #[weak(rename_to = obj)]
                    self.obj(),
                    #[strong]
                    predefined,
                    move |button| {
                        let is_active = button.is_active();

                        Handler::run(glib::clone!(#[strong] predefined, async move {
                            obj.imp().on_suggested_toggle(predefined, is_active).await
                        }))
                    }
                ));
            }
        }

        fn fill_unreadable(&self, config: &crate::config::Backup) {
            self.unreadable_paths.set_visible(false);

            let exclude = &config.exclude;

            let histories = BACKUP_HISTORY.load();
            // If the history is missing we don't have any suggested excludes and shouldn't fail
            let suggested_excludes = histories.active().ok().and_then(|history| {
                history.suggested_excludes_with_reason(
                    config::history::SuggestedExcludeReason::PermissionDenied,
                )
            });

            let Some(suggested_excludes) = suggested_excludes else {
                return;
            };

            for suggested in suggested_excludes {
                // We have at least one entry
                self.unreadable_paths.set_visible(true);

                let add_button = gtk::CheckButton::builder()
                    .tooltip_text(gettext("Add Exclusion Rule"))
                    .valign(gtk::Align::Center)
                    .active(exclude.contains(suggested))
                    .build();

                let row = adw::ActionRow::builder()
                    .title(suggested.description())
                    .activatable_widget(&add_button)
                    .build();

                row.add_prefix(&add_button);

                self.unreadable_paths.add(&row);

                add_button.connect_toggled(glib::clone!(
                    #[strong]
                    suggested,
                    move |button| {
                        Handler::run(glib::clone!(
                            #[strong]
                            suggested,
                            #[weak]
                            button,
                            #[upgrade_or]
                            Ok(()),
                            async move {
                                BACKUP_CONFIG
                                    .try_update(move |settings| {
                                        let active = settings.active_mut()?;

                                        if button.is_active() {
                                            active.exclude.insert(suggested.clone());
                                        } else {
                                            active.exclude.remove(&suggested.clone());
                                        }

                                        Ok(())
                                    })
                                    .await?;

                                main_ui().page_detail().backup_page().refresh()?;
                                Ok(())
                            }
                        ));
                    }
                ));
            }
        }

        /// Find the common ancestor of all included folders
        async fn exclude_base_folder(&self) -> Result<gio::File> {
            let includes = self
                .config
                .get()
                .expect("construct_only property must be set")
                .include_dirs();

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

        pub async fn exclude_folder(&self) -> Result<()> {
            let chooser = gtk::FileDialog::builder()
                .initial_folder(&self.exclude_base_folder().await?)
                .title(gettext("Exclude Folders"))
                .accept_label(gettext("Select"))
                .modal(true)
                .build();

            let paths = ui::utils::paths_from_model(Some(
                chooser
                    .select_multiple_folders_future(Some(&main_ui().window()))
                    .await
                    .map_err(|err| match err.kind::<gtk::DialogError>() {
                        Some(gtk::DialogError::Cancelled | gtk::DialogError::Dismissed) => {
                            Error::UserCanceled
                        }
                        _ => Message::short(err.to_string()).into(),
                    })?,
            ))?;

            BACKUP_CONFIG
                .try_update(|settings| {
                    for path in &paths {
                        settings
                            .active_mut()?
                            .exclude
                            .insert(config::Exclude::from_pattern(config::Pattern::path_prefix(
                                path,
                            )));
                    }
                    Ok(())
                })
                .await?;

            main_ui().page_detail().backup_page().refresh()?;
            Ok(())
        }

        #[template_callback]
        pub async fn on_exclude_folder(&self) {
            self.obj().close();
            self.exclude_folder()
                .await
                .handle_transient_for(&*self.obj())
                .await;
        }

        pub async fn exclude_file(&self) -> Result<()> {
            let chooser = gtk::FileDialog::builder()
                .initial_folder(&self.exclude_base_folder().await?)
                .title(gettext("Exclude Files"))
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

            BACKUP_CONFIG
                .try_update(|settings| {
                    for path in &paths {
                        settings
                            .active_mut()?
                            .exclude
                            .insert(config::Exclude::from_pattern(
                                config::Pattern::path_full_match(path),
                            ));
                    }
                    Ok(())
                })
                .await?;

            main_ui().page_detail().backup_page().refresh()?;
            Ok(())
        }

        #[template_callback]
        pub async fn on_exclude_file(&self) {
            self.obj().close();
            self.exclude_file()
                .await
                .handle_transient_for(&*self.obj())
                .await;
        }

        pub(super) fn exclude_pattern(
            &self,
            old_exclude: Option<config::Exclude<{ config::RELATIVE }>>,
        ) {
            self.pattern_page.set_can_pop(old_exclude.is_none());
            self.navigation_view.push(&*self.pattern_page);

            if let Some(config::Exclude::Pattern(ref pattern)) = old_exclude {
                self.pattern_add_button.set_label(&gettext("Save"));
                self.pattern.set_text(&pattern.pattern().to_string_lossy());

                match pattern {
                    config::Pattern::Fnmatch(_) => self.pattern_type.set_selected(0),
                    config::Pattern::RegularExpression(_) => self.pattern_type.set_selected(1),
                    _ => {}
                }
            } else {
                self.pattern_add_button.set_label(&gettext("Add"));
            }

            self.edit_exclude.replace(old_exclude);
        }

        #[template_callback]
        fn on_exclude_pattern(&self) {
            self.exclude_pattern(None);
        }

        async fn add_pattern(&self) -> Result<()> {
            let selected = self.pattern_type.selected();
            let pattern = self.pattern.text();

            let exclude = config::Exclude::from_pattern(match selected {
                // FIXME: Manual construction
                0 => Ok(config::Pattern::fnmatch(pattern.as_str())),
                1 => config::Pattern::from_regular_expression(pattern)
                    .err_to_msg(gettext("Invalid Regular Expression")),
                // Not translated because this should not happen
                _ => Err(Message::short("No valid pattern type selected").into()),
            }?);

            BACKUP_CONFIG
                .try_update(move |config| {
                    let active = config.active_mut()?;

                    if let Some(edit_exclude) = &*self.edit_exclude.borrow() {
                        active.exclude.remove(edit_exclude);
                    }

                    active.exclude.insert(exclude.clone());

                    Ok(())
                })
                .await?;

            self.obj().close();
            App::default()
                .main_window()
                .page_detail()
                .backup_page()
                .refresh()?;

            Ok(())
        }

        #[template_callback]
        pub async fn on_add_pattern_button_clicked(&self) {
            self.add_pattern()
                .await
                .handle_transient_for(&*self.obj())
                .await;
        }

        async fn on_suggested_toggle(
            &self,
            predefined: config::exclude::Predefined,
            active: bool,
        ) -> Result<()> {
            // TODO: store config id in dialog
            let mut exclude: BTreeSet<config::Exclude<{ config::RELATIVE }>> =
                BACKUP_CONFIG.load().active()?.exclude.clone();

            if active {
                exclude.insert(config::Exclude::from_predefined(predefined));
            } else {
                exclude.retain(|x| matches!(x, config::Exclude::Predefined(p) if *p != predefined));
            }

            BACKUP_CONFIG
                .try_update(move |settings| {
                    settings.active_mut()?.exclude.clone_from(&exclude);
                    Ok(())
                })
                .await?;

            main_ui().page_detail().backup_page().refresh()?;

            Ok(())
        }
    }
}

glib::wrapper! {
    pub struct ExcludeDialog(ObjectSubclass<imp::ExcludeDialog>)
    @extends gtk::Widget, adw::Dialog,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ExcludeDialog {
    pub fn new(config: &crate::config::Backup) -> Self {
        glib::Object::builder().property("config", config).build()
    }

    pub fn present_edit_exclude(
        &self,
        widget: &impl IsA<gtk::Widget>,
        pattern: config::Exclude<{ config::RELATIVE }>,
    ) {
        self.imp().exclude_pattern(Some(pattern));
        self.present(Some(widget));
    }
}
