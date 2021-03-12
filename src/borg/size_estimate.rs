use super::*;
use crate::config;
use crate::prelude::*;

fn pathmatch(entry: &walkdir::DirEntry, pattern: &config::Pattern) -> bool {
    match pattern {
        config::Pattern::PathPrefix(path) => entry.path() == path,
    }
}

pub fn recalculate(config: &config::Backup, mut communication: Communication) {
    communication.instruction = Default::default();

    let estimated_size = calculate(&config, &communication);

    if estimated_size.is_some() {
        communication.status.update(move |status| {
            status.estimated_size = estimated_size.clone();
        });
    }
}

/// Estimate backup size
///
/// Returns the total size of the backup and the size of all created/modified files.
/// Using `u64` is sufficient for several exabytes.

pub fn calculate(config: &config::Backup, communication: &Communication) -> Option<SizeEstimate> {
    debug!("Estimating backup size");

    // TODO: we need the last backup that not failed
    let last_run = BACKUP_HISTORY
        .load()
        .get_result(&config.id)
        .ok()
        .and_then(|x| x.last_completed.as_ref())
        .map(|x| x.end.into())
        .unwrap_or_else(|| std::time::SystemTime::UNIX_EPOCH);

    let mut exclude = config.exclude_dirs_internal();

    // Exclude .cache/borg
    if let Some(cache_dir) = glib::get_user_cache_dir() {
        exclude.push(config::Pattern::PathPrefix(
            cache_dir.join(std::path::Path::new("borg")),
        ));
    }

    let is_not_exluded = |e: &walkdir::DirEntry| !exclude.iter().any(|x| pathmatch(e, x));

    let mut size_total = 0;
    let mut size_touched = 0;

    for dir in config.include_dirs() {
        for entry in walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_entry(is_not_exluded)
            .filter_map(std::result::Result::ok)
        {
            if Instruction::Nothing != **communication.instruction.load() {
                return None;
            }

            if entry.file_type().is_dir() {
                // Empirical value for the space that borg needs
                size_total += 109;
            } else if let Ok(metadata) = entry.metadata() {
                size_total += metadata.len();

                // check created and modified date against last backup date
                if metadata
                    .modified()
                    .map(|date| date >= last_run)
                    .unwrap_or_default()
                    || metadata
                        .created()
                        .map(|date| date >= last_run)
                        .unwrap_or_default()
                {
                    size_touched += metadata.len();
                }
            }
        }
    }

    debug!(
        "Estimated size: total {} created/modified {}",
        &size_total, &size_touched
    );

    Some(SizeEstimate {
        total: size_total,
        changed: size_touched,
    })
}
