use adw::prelude::*;
use ui::prelude::*;

use crate::{config, ui};
use std::path::PathBuf;

pub enum LocationTag {
    Location(PathBuf),
    Pattern(config::Pattern),
}

impl LocationTag {
    pub const fn from_path(path: PathBuf) -> Self {
        Self::Location(path)
    }

    pub fn from_pattern(pattern: config::Pattern) -> Self {
        match pattern {
            config::Pattern::PathPrefix(path) => Self::Location(path),
            pattern => Self::Pattern(pattern),
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::Location(path) => {
                if path.iter().next().is_none() {
                    gettext("Home")
                } else {
                    path.to_string_lossy().to_string()
                }
            }
            Self::Pattern(pattern) => pattern.description(),
        }
    }

    pub fn icon(&self) -> Option<gtk::Image> {
        match self {
            Self::Location(path) => ui::utils::file_symbolic_icon(&config::absolute(path)),
            Self::Pattern(_) => Some(gtk::Image::from_icon_name("folder-saved-search-symbolic")),
        }
    }

    pub fn build(&self) -> gtk::Box {
        let incl = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .valign(gtk::Align::Center)
            .spacing(4)
            .build();
        incl.add_css_class("tag");

        if let Some(icon) = self.icon() {
            incl.append(&icon);
        }

        let label = gtk::Label::builder()
            .label(&self.label())
            .ellipsize(gtk::pango::EllipsizeMode::Middle)
            .natural_wrap_mode(gtk::NaturalWrapMode::None)
            .build();
        incl.append(&label);

        incl
    }
}
