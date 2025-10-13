mod common;

use pika_backup::borg::size_estimate::*;
use pika_backup::borg::status::SizeEstimate;
use pika_backup::config;
use pika_backup::config::{Exclude, Pattern};

fn config(include: &[&str], exclude: &[Exclude<{ config::RELATIVE }>]) -> config::Backup {
    let mut config = common::config(std::path::Path::new("backup_data"));

    for path in include {
        config.include.insert(total(path));
    }

    for pattern in exclude {
        config.exclude.insert(pattern.clone());
    }

    config
}

fn calc(config: &config::Backup) -> SizeEstimate {
    calculate(config, &config::Histories::default(), &Default::default()).unwrap()
}

fn total(path: &str) -> std::path::PathBuf {
    std::env::current_dir().unwrap().join("tests").join(path)
}

#[test]
fn include_duplicates() {
    let complete = calc(&config(&["backup_data"], &[]));
    let duplicates = calc(&config(&["backup_data", "backup_data/Downloads"], &[]));

    assert!(complete.total > 0);
    assert_eq!(complete.total, duplicates.total);
}

#[test]
fn simple_exclude() {
    let complete = calc(&config(
        &["backup_data"],
        &[pp("backup_data/Downloads"), pp("backup_data/h1")],
    ));
    let specific = calc(&config(&["backup_data/Documents"], &[]));

    assert!(complete.total > 0);
    assert_eq!(complete.total, specific.total + DIRECTORY_SIZE);
}

#[test]
fn simple_regex_exclude() {
    let pp_exclude = calc(&config(&["backup_data"], &[pp("backup_data/Downloads")]));
    let re_exclude = calc(&config(&["backup_data"], &[re("/Downloads")]));
    let complex_exclude = calc(&config(&["backup_data"], &[re(".*/Downloads.*")]));

    assert!(pp_exclude.total > 0);
    assert_eq!(pp_exclude.total, re_exclude.total);
    assert_eq!(pp_exclude.total, complex_exclude.total);
}

fn pp(pp: &str) -> Exclude<{ config::RELATIVE }> {
    Exclude::from_pattern(Pattern::PathPrefix(total(pp)))
}

fn re(re: &str) -> Exclude<{ config::RELATIVE }> {
    Exclude::from_pattern(Pattern::RegularExpression(regex::Regex::new(re).unwrap()))
}
