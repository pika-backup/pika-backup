use std::collections::BTreeMap;
use std::iter::FromIterator;

use crate::borg;

use super::error;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Space {
    pub size: u64,
    pub used: u64,
    pub avail: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RepoCache {
    pub repo_id: borg::RepoId,
    pub archives: Option<BTreeMap<borg::ArchiveName, borg::ListArchive>>,
    #[serde(skip)]
    pub reloading: bool,
    pub space: Option<Space>,
}

impl RepoCache {
    fn new(repo_id: &borg::RepoId) -> Self {
        Self {
            repo_id: repo_id.clone(),
            archives: None,
            reloading: false,
            space: None,
        }
    }

    pub fn get(repo_id: &borg::RepoId) -> Self {
        debug!("Try loading repo cache from file");

        let repo_cache: Option<Self> = std::fs::File::open(Self::path(repo_id))
            .ok()
            .and_then(|f| serde_json::from_reader(f).ok());

        if let Some(cache) = repo_cache {
            cache
        } else {
            Self::new(repo_id)
        }
    }

    pub fn write(&self) -> Result<(), error::RepoCache> {
        match std::fs::DirBuilder::new()
            .recursive(true)
            .create(crate::utils::cache_dir())
            .and_then(|_| std::fs::File::create(Self::path(&self.repo_id)))
        {
            Ok(file) => {
                serde_json::ser::to_writer(&file, &self).map_err(error::RepoCache::WriteError)
            }
            Err(err) => Err(error::RepoCache::ReadError(err)),
        }
    }

    pub fn path(repo_id: &crate::borg::RepoId) -> std::path::PathBuf {
        [crate::utils::cache_dir(), repo_id.as_str().into()]
            .iter()
            .collect()
    }

    pub fn archives_sorted_by_date(&self) -> Vec<(borg::ArchiveName, borg::ListArchive)> {
        if let Some(archives) = self.archives.clone() {
            let mut vec = Vec::from_iter(archives);
            vec.sort_by(|x, y| y.1.start.cmp(&x.1.start));
            vec
        } else {
            Vec::new()
        }
    }
}
