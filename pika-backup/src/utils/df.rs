//! Disk space information

use async_process as process;
use common::config;
use enclose::enclose;
use gio::prelude::*;
use quick_error::quick_error;
use serde::{Deserialize, Serialize};
use smol::prelude::*;

use crate::prelude::*;
use crate::utils::repo_cache::RepoCache;

type Result<T> = std::result::Result<T, Error>;

pub async fn cached_or_lookup(config: &config::Backup) -> Option<Space> {
    let cached = RepoCache::get(&config.repo_id).space;

    match &config.repo {
        config::Repository::Local(_) => {
            let lookup = lookup_and_cache(config).await;
            if lookup.is_ok() { lookup.ok() } else { cached }
        }
        config::Repository::Remote(_) => {
            if cached.is_some() {
                cached
            } else {
                lookup_and_cache(config).await.ok()
            }
        }
    }
}

pub async fn lookup_and_cache(config: &config::Backup) -> Result<Space> {
    let space = match &config.repo {
        config::Repository::Local(repo) => local(&repo.path()).await,
        config::Repository::Remote(repo) => remote(&repo.uri).await,
    }?;

    REPO_CACHE.update(enclose!((config, space) move |cache| {
        cache
            .entry(config.repo_id.clone())
            .or_insert_with_key(RepoCache::new)
            .space = Some(space.clone());
    }));
    let _ignore = RepoCache::write(&config.repo_id);

    Ok(space)
}

fn sftp_path_normalize(path: &str) -> String {
    if path.starts_with("/~") {
        path.replace('~', ".")
            .get(1..)
            .unwrap_or_default()
            .to_string()
    } else {
        path.to_string()
    }
}

pub async fn remote(server: &str) -> Result<Space> {
    let original_uri = glib::Uri::parse(server, glib::UriFlags::NONE)?;

    // If the remote uses SSH with the SSH scheme and a port was specified we use
    // that port
    let port = if original_uri.scheme() == "ssh" {
        original_uri.port()
    } else {
        -1
    };

    let connect_url = glib::Uri::build(
        glib::UriFlags::NONE,
        "sftp",
        original_uri.userinfo().as_ref().map(|x| x.as_str()),
        original_uri.host().as_ref().map(|x| x.as_str()),
        port,
        "",
        None,
        None,
    );

    // just hope that the home path is the same as the default path
    let path = sftp_path_normalize(&original_uri.path());

    tracing::debug!("sftp connect to '{}'", connect_url.to_str());

    let mut child = process::Command::new("sftp")
        .args(["-b", "-", &connect_url.to_str()])
        .stdin(process::Stdio::piped())
        .stderr(process::Stdio::piped())
        .stdout(process::Stdio::piped())
        .spawn()?;
    let mut stdin = child.stdin.take().ok_or("STDIN not available.")?;

    // this might fail but we don't care since output goes to STDERR
    tracing::debug!("sftp: try to change to dir {:?}", path);
    stdin
        .write_all(format!("cd {}\n", shell_words::quote(&path)).as_bytes())
        .await?;

    stdin.write_all(b"df\nexit\n").await?;

    let out = child.output().await?;

    tracing::debug!(
        "sftp output:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let df: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .nth(3)
        .ok_or("Fourth line missing in output.")?
        .split_whitespace()
        .map(str::to_string)
        .collect();

    // df gives us kb not bytes
    Ok(Space {
        size: 1024 * df.get(0).ok_or("First column missing.")?.parse::<u64>()?,
        used: 1024 * df.get(1).ok_or("Second column missing.")?.parse::<u64>()?,
        avail: 1024 * df.get(2).ok_or("Third column missing.")?.parse::<u64>()?,
    })
}

pub async fn local(root: &std::path::Path) -> Result<Space> {
    let fsinfo = gio::File::for_path(root)
        .query_filesystem_info_future("*", Default::default())
        .await?;

    Ok(Space {
        size: fsinfo.attribute_uint64(gio::FILE_ATTRIBUTE_FILESYSTEM_SIZE),
        used: fsinfo.attribute_uint64(gio::FILE_ATTRIBUTE_FILESYSTEM_USED),
        avail: fsinfo.attribute_uint64(gio::FILE_ATTRIBUTE_FILESYSTEM_FREE),
    })
}

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        GLib(err: glib::Error) { from() }
        ParseInt(err: std::num::ParseIntError) { from() }
        StdIo(err: std::io::Error) { from() }
        Other(err: String) { from(err: &str) -> (err.to_string()) }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Space {
    pub size: u64,
    pub used: u64,
    pub avail: u64,
}

#[test]
fn test_uri_normalize() {
    let uri = glib::Uri::parse("ssh://borg@example.net/~/backup", glib::UriFlags::NONE).unwrap();
    let path = sftp_path_normalize(&uri.path());
    assert_eq!(path, "./backup");

    let uri = glib::Uri::parse("ssh://borg@example.net/mnt/backup", glib::UriFlags::NONE).unwrap();
    let path = sftp_path_normalize(&uri.path());
    assert_eq!(path, "/mnt/backup");
}
