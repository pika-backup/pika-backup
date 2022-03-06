/*!
Borg output to STDOUT with `--json` flag.
*/

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Ord, Eq, PartialOrd, PartialEq)]
pub struct RepoId(String);

impl RepoId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl glib::ToVariant for RepoId {
    fn to_variant(&self) -> glib::Variant {
        self.as_str().to_variant()
    }
}

impl glib::FromVariant for RepoId {
    fn from_variant(variant: &glib::Variant) -> Option<Self> {
        let id = glib::FromVariant::from_variant(variant)?;
        Some(RepoId::new(id))
    }
}

impl glib::StaticVariantType for RepoId {
    fn static_variant_type() -> std::borrow::Cow<'static, glib::VariantTy> {
        String::static_variant_type()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Ord, Eq, PartialOrd, PartialEq)]
pub struct ArchiveId(String);

impl ArchiveId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Ord, Eq, PartialOrd, PartialEq)]
pub struct ArchiveName(String);

impl ArchiveName {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Stats {
    pub archive: NewArchive,
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
