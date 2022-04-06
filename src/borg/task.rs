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
pub struct List {}

impl Task for List {
    type Info = ();
    type Return = Vec<super::ListArchive>;

    fn name() -> String {
        gettext("Refreshing Archive List")
    }
}
