use adw::prelude::*;

use crate::config;
use std::path::PathBuf;

pub enum LocationTag {
    Location(PathBuf),
    Exclude(config::Exclude<{ config::RELATIVE }>),
}

impl LocationTag {
    pub const fn from_path(path: PathBuf) -> Self {
        Self::Location(path)
    }

    pub fn from_exclude(exclude: config::Exclude<{ config::RELATIVE }>) -> Self {
        Self::Exclude(exclude)
    }

    fn label(&self) -> String {
        match self {
            Self::Location(path) => config::display_path(path),
            Self::Exclude(exclude) => exclude.description(),
        }
    }

    fn icon(&self) -> Option<gtk::Image> {
        match self {
            Self::Location(path) => crate::utils::file_symbolic_icon(&config::absolute(path)),
            Self::Exclude(exclude) => exclude.symbolic_icon(),
        }
        .map(|x| gtk::Image::from_gicon(&x))
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
