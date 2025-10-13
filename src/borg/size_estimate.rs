use std::collections::BTreeSet;

use super::*;
use crate::config;
use crate::prelude::*;

/// Empirical value for the space that borg needs
pub static DIRECTORY_SIZE: u64 = 109;

struct Exclude {
    exclude: BTreeSet<config::Exclude<{ config::ABSOLUTE }>>,
}

impl Exclude {
    pub fn borg_cache() -> std::path::PathBuf {
        glib::user_cache_dir().join(std::path::Path::new("borg"))
    }

    pub fn new(mut exclude: BTreeSet<config::Exclude<{ config::ABSOLUTE }>>) -> Self {
        exclude.insert(config::Exclude::from_pattern(config::Pattern::path_prefix(
            Self::borg_cache(),
        )));

        Self { exclude }
    }

    pub fn is_included(&self, entry: &walkdir::DirEntry) -> bool {
        !self
            .exclude
            .iter()
            .any(|pattern| pattern.is_match(entry.path()))
    }
}

/// Estimate backup size
///
/// Returns the total size of the backup and the size of all created/modified
/// files. Using `u64` is sufficient for several exabytes.
pub fn calculate(
    config: &config::Backup,
    history: &config::history::Histories,
    communication: &Communication<task::Create>,
) -> Option<SizeEstimate> {
    debug!("Estimating backup size");

    // datetime of last completed backup
    let last_run = history
        .try_get(&config.id)
        .ok()
        .and_then(|x| x.last_completed());

    let last_run_date = last_run
        .map(|x| x.end.into())
        .unwrap_or_else(|| std::time::SystemTime::UNIX_EPOCH);

    let exclude = Exclude::new(config.exclude_dirs_internal());

    let duplicate_check = {
        let include = config.include_dirs();
        move |entry: &std::path::PathBuf| {
            !include
                .iter()
                .any(|other| entry != other && entry.starts_with(other))
        }
    };
    let include = config.include_dirs().into_iter().filter(duplicate_check);

    let exclude_previously = Exclude::new(last_run.map(|x| x.exclude.clone()).unwrap_or_default());
    let include_previously = last_run.map(|x| x.include.clone()).unwrap_or_default();

    let mut size_total = 0;
    let mut size_touched = 0;
    let mut unreadable_paths = Vec::new();

    for dir in include {
        for entry_result in walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_entry(|entry| exclude.is_included(entry))
        {
            match entry_result {
                Ok(entry) => {
                    if matches!(**communication.instruction.load(), Instruction::Abort(_)) {
                        return None;
                    }

                    if entry.file_type().is_dir() {
                        size_total += DIRECTORY_SIZE;
                    } else if let Ok(metadata) = entry.metadata() {
                        size_total += metadata.len();

                        // Workaround for rust std assertion about broken mtime / ctime on btrfs
                        let result = std::panic::catch_unwind(|| {
                            // check if file is new/modified since last backup
                            if metadata
                                .modified()
                                .map(|date| date >= last_run_date)
                                .unwrap_or_default()
                                || metadata
                                    .created()
                                    .map(|date| date >= last_run_date)
                                    .unwrap_or_default()
                                || !exclude_previously.is_included(&entry)
                                || !include_previously
                                    .iter()
                                    .any(|p| entry.path().starts_with(p))
                            {
                                metadata.len()
                            } else {
                                0
                            }
                        });

                        match result {
                            Ok(size) => size_touched += size,
                            Err(err) => error!(
                                "FILESYSTEM BUG: mtime/ctime has invalid value for path {:?}. Backtrace: {err:?}",
                                entry.path()
                            ),
                        };
                    }
                }
                Err(err) => {
                    if let (Some(path), Some(io_error)) = (err.path(), err.io_error())
                        && io_error.kind() == std::io::ErrorKind::PermissionDenied
                    {
                        unreadable_paths.push(path.to_path_buf());
                    }
                }
            }
        }
    }

    debug!(
        "Estimated size: total {} created/modified {}",
        &size_total, &size_touched
    );
    debug!(
        "Number of unreablable files/directories: {}",
        unreadable_paths.len()
    );
    trace!("Unreadable files/directories: {:#?}", unreadable_paths);

    Some(SizeEstimate {
        total: size_total,
        changed: size_touched,
        unreadable_paths,
    })
}
