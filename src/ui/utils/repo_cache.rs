use crate::ui::prelude::*;

use std::collections::BTreeMap;
use std::iter::FromIterator;

use crate::borg;
use crate::ui;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RepoCache {
    pub repo_id: borg::RepoId,
    pub archives: Option<BTreeMap<borg::ArchiveName, borg::Archive>>,
    #[serde(skip)]
    pub reloading: bool,
    pub space: Option<ui::utils::df::Space>,
}

impl RepoCache {
    pub fn new(repo_id: &borg::RepoId) -> Self {
        Self {
            repo_id: repo_id.clone(),
            archives: None,
            reloading: false,
            space: None,
        }
    }

    pub fn get(repo_id: &borg::RepoId) -> RepoCache {
        if let Some(repo_archives) = REPO_CACHE.load().get(repo_id) {
            debug!("Repo cache already loaded from file");
            repo_archives.clone()
        } else {
            debug!("Try loading repo cache from file");

            let repo_cache: Option<RepoCache> = std::fs::File::open(Self::path(repo_id))
                .ok()
                .and_then(|f| serde_json::from_reader(f).ok());

            if let Some(cache) = repo_cache {
                REPO_CACHE.update(enclose!(
                    (repo_id, cache) | ra | {
                        ra.insert(repo_id, cache);
                    }
                ));
                cache
            } else {
                Self::new(repo_id)
            }
        }
    }

    pub fn write(repo_id: &borg::RepoId) -> Result<()> {
        match std::fs::DirBuilder::new()
            .recursive(true)
            .create(ui::utils::cache_dir())
            .and_then(|_| std::fs::File::create(Self::path(repo_id)))
        {
            Ok(file) => serde_json::ser::to_writer(&file, &REPO_CACHE.load().get(repo_id))
                .err_to_msg(gettext("Failed to save cache.")),
            Err(err) => Err(Message::new("Failed to open cache file.", err).into()),
        }
    }

    pub fn path(repo_id: &crate::borg::RepoId) -> std::path::PathBuf {
        [super::cache_dir(), repo_id.as_str().into()]
            .iter()
            .collect()
    }

    pub fn archives_sorted_by_date(&self) -> Vec<(borg::ArchiveName, borg::Archive)> {
        if let Some(archives) = self.archives.clone() {
            let mut vec = Vec::from_iter(archives);
            vec.sort_by(|x, y| y.1.start.cmp(&x.1.start));
            vec
        } else {
            Vec::new()
        }
    }
}
