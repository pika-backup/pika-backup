//use std::io::Write;
use gio::prelude::*;

use async_process as process;
use futures::AsyncWriteExt;

pub async fn remote(server: &str) -> Result<Space, Error> {
    let mut url = url::Url::parse(server)?;
    let _ignore = url.set_scheme("sftp");
    url.set_path("");

    debug!("sftp connect to '{}'", url.as_str());

    let mut child = process::Command::new("sftp")
        .arg(url.as_str())
        .stdin(process::Stdio::piped())
        .stderr(process::Stdio::piped())
        .stdout(process::Stdio::piped())
        .spawn()?;
    let mut stdin = child.stdin.take().ok_or("STDIN not available.")?;

    stdin.write_all(b"df\nexit\n").await?;

    let out = child.output().await?;

    debug!(
        "sftp output:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let df: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .nth(2)
        .ok_or("Third line missing in output.")?
        .split_whitespace()
        .map(str::to_string)
        .collect();

    Ok(Space {
        size: df.get(0).ok_or("First column missing.")?.parse()?,
        used: df.get(1).ok_or("Second column missing.")?.parse()?,
        avail: df.get(2).ok_or("Third column missing.")?.parse()?,
    })
}

pub fn local(root: &gio::File) -> Result<Space, glib::Error> {
    let none: Option<&gio::Cancellable> = None;
    let fsinfo = root.query_filesystem_info("*", none)?;
    Ok(Space {
        size: fsinfo.get_attribute_uint64(&gio::FILE_ATTRIBUTE_FILESYSTEM_SIZE),
        used: fsinfo.get_attribute_uint64(&gio::FILE_ATTRIBUTE_FILESYSTEM_USED),
        avail: fsinfo.get_attribute_uint64(&gio::FILE_ATTRIBUTE_FILESYSTEM_FREE),
    })
}

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        ParseInt(err: std::num::ParseIntError) { from() }
        StdIo(err: std::io::Error) { from() }
        Url(err: url::ParseError) { from() }
        Other(err: String) { from(err: &str) -> (err.to_string()) }
    }
}

#[derive(Debug)]
pub struct Space {
    pub size: u64,
    pub used: u64,
    pub avail: u64,
}
