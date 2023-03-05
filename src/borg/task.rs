use crate::prelude::*;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Kind {
    Create,
    Mount,
    Prune,
    PruneInfo,
    Compact,
    Delete,
    List,
}

pub trait Task: Clone + Default + Send + Sync + 'static {
    //type Status: Clone + Default + Send + Sync;
    type Info: Clone + Default + Send + Sync;
    type Return: Clone + Send;

    const KIND: Kind;

    fn name() -> String;
}

#[derive(Clone, Default)]
pub struct Create {}

impl Task for Create {
    type Info = super::status::Status;
    type Return = super::Stats;

    const KIND: Kind = Kind::Create;

    fn name() -> String {
        gettext("Backing up Data")
    }
}

#[derive(Clone, Default)]
pub struct Mount {}

impl Task for Mount {
    type Info = ();
    type Return = ();

    const KIND: Kind = Kind::Mount;

    fn name() -> String {
        gettext("Mounting Backup Archives")
    }
}

#[derive(Clone, Default)]
pub struct Prune {}

impl Task for Prune {
    type Info = ();
    type Return = ();

    const KIND: Kind = Kind::Prune;

    fn name() -> String {
        gettext("Removing old Archives")
    }
}

#[derive(Clone, Default)]
pub struct PruneInfo {}

impl Task for PruneInfo {
    type Info = ();
    type Return = super::PruneInfo;

    const KIND: Kind = Kind::PruneInfo;

    fn name() -> String {
        gettext("Identifying old Archives")
    }
}

#[derive(Clone, Default)]
pub struct Compact {}

impl Task for Compact {
    type Info = ();
    type Return = ();

    const KIND: Kind = Kind::Compact;

    fn name() -> String {
        gettext("Reclaiming Free Space")
    }
}

#[derive(Clone, Default)]
pub struct Delete {
    archive_name: Option<String>,
}

impl Delete {
    pub fn set_archive_name(&mut self, archive_name: Option<String>) -> &mut Self {
        self.archive_name = archive_name;
        self
    }

    pub fn archive_name(&self) -> Option<String> {
        self.archive_name.clone()
    }
}

impl Task for Delete {
    type Info = ();
    type Return = ();

    const KIND: Kind = Kind::Delete;

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

    const KIND: Kind = Kind::List;

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
