use adw::prelude::*;
use ui::prelude::*;

use crate::{config, ui};
use std::path::PathBuf;

pub struct LocationTag {
    path: PathBuf,
}

impl LocationTag {
    pub const fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn build(&self) -> gtk::Box {
        let incl = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .valign(gtk::Align::Center)
            .spacing(4)
            .build();
        incl.add_css_class("tag");

        if let Some(icon) = ui::utils::file_symbolic_icon(&config::absolute(&self.path)) {
            incl.append(&icon);
        }

        let path_str = if self.path.iter().next().is_none() {
            gettext("Home")
        } else {
            self.path.to_string_lossy().to_string()
        };

        let label = gtk::Label::builder()
            .label(&path_str)
            .ellipsize(gtk::pango::EllipsizeMode::Middle)
            .natural_wrap_mode(gtk::NaturalWrapMode::None)
            .build();
        incl.append(&label);

        incl
    }
}
