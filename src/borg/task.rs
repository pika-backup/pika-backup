use crate::prelude::*;

use crate::config::UserScriptKind;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Kind {
    Create,
    Mount,
    Prune,
    PruneInfo,
    Compact,
    Check,
    Delete,
    List,

    // A custom script from the user
    UserScript,
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
pub struct Check {
    verify_data: bool,
    repair: bool,
}

impl Check {
    pub fn verify_data(&self) -> bool {
        self.verify_data
    }

    pub fn set_verify_data(&mut self, verify_data: bool) {
        self.verify_data = verify_data;
    }

    pub fn repair(&self) -> bool {
        self.repair
    }

    pub fn set_repair(&mut self, repair: bool) {
        self.repair = repair;
    }
}

impl Task for Check {
    type Info = ();
    type Return = ();

    const KIND: Kind = Kind::Check;

    fn name() -> String {
        gettext("Checking Archives Integrity")
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

#[derive(Clone, Default, Debug)]
pub struct UserScript {
    kind: Option<UserScriptKind>,
    run_info: Option<crate::config::history::RunInfo>,
}

impl UserScript {
    pub fn set_kind(&mut self, kind: UserScriptKind) {
        self.kind = Some(kind);
    }

    pub fn kind(&self) -> Option<UserScriptKind> {
        self.kind.clone()
    }

    pub fn set_run_info(&mut self, run_info: Option<crate::config::history::RunInfo>) {
        self.run_info = run_info;
    }

    pub fn run_info(&self) -> Option<&crate::config::history::RunInfo> {
        self.run_info.as_ref()
    }
}

impl Task for UserScript {
    type Info = ();
    type Return = ();

    const KIND: Kind = Kind::UserScript;

    fn name() -> String {
        gettext("Running Custom Shell Command")
    }
}
