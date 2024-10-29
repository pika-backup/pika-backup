use crate::config;
use crate::ui::prelude::*;

use glib::prelude::*;
use glib::subclass::prelude::*;
use std::cell::RefCell;

#[derive(Debug, Clone)]
pub enum PrunePreset {
    KeepMany,
    KeepSome,
    Custom,
}

impl PrunePreset {
    pub fn name(&self) -> String {
        match self {
            Self::KeepMany => gettext("Keep Many"),
            Self::KeepSome => gettext("Keep Some"),
            Self::Custom => gettext("Custom"),
        }
    }

    pub fn keep(&self) -> Option<config::Keep> {
        match self {
            Self::KeepMany => Some(config::Keep::default()),
            Self::KeepSome => Some(config::Keep {
                hourly: 24,
                daily: 7,
                weekly: 2,
                monthly: 6,
                yearly: 0,
            }),
            Self::Custom => None,
        }
    }

    pub fn matching(keep: &config::Keep) -> Self {
        for preset in Self::list() {
            if Some(keep) == preset.keep().as_ref() {
                return preset;
            }
        }

        Self::Custom
    }

    pub fn list() -> Vec<Self> {
        vec![Self::KeepMany, Self::KeepSome, Self::Custom]
    }
}

impl Default for PrunePreset {
    fn default() -> Self {
        Self::Custom
    }
}

glib::wrapper! {
    pub struct PrunePresetObject(ObjectSubclass<imp::PrunePresetObject>);
}

impl PrunePresetObject {
    pub fn new(preset: PrunePreset) -> Self {
        let new: Self = glib::Object::new();
        new.imp().preset.replace(preset);
        new
    }

    pub fn preset(&self) -> PrunePreset {
        self.imp().preset.borrow().clone()
    }

    pub fn list_store() -> gio::ListStore {
        let model = gio::ListStore::with_type(Self::static_type());

        for elem in PrunePreset::list() {
            model.append(&Self::new(elem));
        }

        model
    }
}

mod imp {
    use std::marker::PhantomData;

    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type=super::PrunePresetObject)]
    pub struct PrunePresetObject {
        pub preset: RefCell<PrunePreset>,
        #[property(get=Self::name)]
        display: PhantomData<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PrunePresetObject {
        const NAME: &'static str = "PikaBackupPrunePreset";
        type Type = super::PrunePresetObject;
        type ParentType = glib::Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for PrunePresetObject {}

    impl PrunePresetObject {
        fn name(&self) -> String {
            self.preset.borrow().name()
        }
    }
}
