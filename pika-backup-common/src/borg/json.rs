/*!
Borg output to STDOUT with `--json` flag.
*/

use glib::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Ord, Eq, PartialOrd, PartialEq)]
pub struct RepoId(String);

impl RepoId {
    pub const fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl ToVariant for RepoId {
    fn to_variant(&self) -> glib::Variant {
        self.as_str().to_variant()
    }
}

impl FromVariant for RepoId {
    fn from_variant(variant: &glib::Variant) -> Option<Self> {
        let id = FromVariant::from_variant(variant)?;
        Some(Self::new(id))
    }
}

impl StaticVariantType for RepoId {
    fn static_variant_type() -> std::borrow::Cow<'static, glib::VariantTy> {
        String::static_variant_type()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Ord, Eq, PartialOrd, PartialEq)]
pub struct ArchiveId(String);

impl ArchiveId {
    pub const fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Ord, Eq, PartialOrd, PartialEq)]
pub struct ArchiveName(String);

impl ArchiveName {
    pub const fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Stats {
    pub archive: NewArchive,
}

impl Stats {
    pub fn transfer_history_mock(archive: &ListArchive) -> Self {
        Stats {
            archive: NewArchive {
                duration: (archive.end - archive.start).num_seconds() as f64,
                id: archive.id.clone(),
                name: archive.name.clone(),
                stats: NewArchiveSize {
                    compressed_size: 0,
                    deduplicated_size: 0,
                    nfiles: 0,
                    original_size: 0,
                },
            },
        }
    }

    /// Create stats for usage in examples (like shell script tests)
    pub fn new_example() -> Self {
        Stats {
            archive: NewArchive {
                duration: 100.,
                id: ArchiveId::new(
                    "b8fe5b22bc490b12a5b7fd231c8ec8b8cc68805b1cc4cb8a84d643e1e76a89fa".to_string(),
                ),
                name: ArchiveName::new("5adc9f-d6096ee8".to_string()),
                stats: NewArchiveSize {
                    compressed_size: 3085251047,
                    deduplicated_size: 783,
                    nfiles: 783,
                    original_size: 3124637266,
                },
            },
        }
    }

    #[cfg(test)]
    pub fn test_new_mock() -> Self {
        Stats {
            archive: NewArchive {
                duration: 0.,
                id: ArchiveId::new(String::new()),
                name: ArchiveName::new(String::new()),
                stats: NewArchiveSize {
                    compressed_size: 0,
                    deduplicated_size: 0,
                    nfiles: 0,
                    original_size: 0,
                },
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NewArchive {
    pub duration: f64,
    pub id: ArchiveId,
    pub name: ArchiveName,
    pub stats: NewArchiveSize,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NewArchiveSize {
    pub compressed_size: u64,
    pub deduplicated_size: u64,
    pub nfiles: u64,
    pub original_size: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct List {
    pub archives: Vec<ListArchive>,
    pub encryption: Encryption,
    pub repository: Repository,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListArchive {
    pub id: ArchiveId,
    pub name: ArchiveName,
    pub comment: String,
    pub username: String,
    pub hostname: String,
    pub start: chrono::naive::NaiveDateTime,
    pub end: chrono::naive::NaiveDateTime,
    pub command_line: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Info {
    pub archives: Vec<InfoArchive>,
    pub encryption: Encryption,
    pub repository: Repository,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InfoArchive {
    pub id: ArchiveId,
    pub name: ArchiveName,
    pub comment: String,
    pub username: String,
    pub hostname: String,
    pub start: chrono::naive::NaiveDateTime,
    pub end: chrono::naive::NaiveDateTime,
    pub command_line: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Encryption {
    pub mode: String,
    pub keyfile: Option<std::path::PathBuf>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Repository {
    pub id: RepoId,
    pub last_modified: chrono::naive::NaiveDateTime,
    pub location: std::path::PathBuf,
}
