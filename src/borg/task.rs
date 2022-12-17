use crate::prelude::*;

pub trait Task: Clone + Default + Send + Sync + 'static {
    //type Status: Clone + Default + Send + Sync;
    type Info: Clone + Default + Send + Sync;
    type Return: Clone + Send;

    fn name() -> String;
}

#[derive(Clone, Default)]
pub struct Create {}

impl Task for Create {
    type Info = super::status::Status;
    type Return = super::Stats;

    fn name() -> String {
        gettext("Backing up Data")
    }
}

#[derive(Clone, Default)]
pub struct Mount {}

impl Task for Mount {
    type Info = ();
    type Return = ();

    fn name() -> String {
        gettext("Mounting Backup Archives")
    }
}

#[derive(Clone, Default)]
pub struct Prune {}

impl Task for Prune {
    type Info = ();
    type Return = ();

    fn name() -> String {
        gettext("Removing old Archives")
    }
}

#[derive(Clone, Default)]
pub struct PruneInfo {}

impl Task for PruneInfo {
    type Info = ();
    type Return = super::PruneInfo;

    fn name() -> String {
        gettext("Identifying old Archives")
    }
}

#[derive(Clone, Default)]
pub struct Compact {}

impl Task for Compact {
    type Info = ();
    type Return = ();

    fn name() -> String {
        gettext("Compacting Archives")
    }
}

#[derive(Clone, Default)]
pub struct Delete {}

impl Task for Delete {
    type Info = ();
    type Return = ();

    fn name() -> String {
        gettext("Deleting Archive")
    }
}

#[derive(Clone, Default)]
pub struct List {
    pub(super) limit: NumArchives,
}

impl List {
    pub fn set_limit_first(&mut self, limit: u32) -> &mut Self {
        self.limit = NumArchives::First(limit);
        self
    }
}

impl Task for List {
    type Info = ();
    type Return = Vec<super::ListArchive>;

    fn name() -> String {
        gettext("Refreshing Archive List")
    }
}

#[derive(Clone)]
pub(super) enum NumArchives {
    All,
    First(u32),
}

impl Default for NumArchives {
    fn default() -> Self {
        Self::All
    }
}
