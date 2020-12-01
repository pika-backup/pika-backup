use crate::borg;
use chrono::prelude::*;
use gio::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path;
use zeroize::Zeroizing;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BackupConfig {
    #[serde(default)]
    pub config_version: u16,
    pub id: String,
    #[serde(default = "fake_repo_id")]
    pub repo_id: String,
    pub repo: BackupRepo,
    pub encrypted: bool,
    #[serde(default)]
    pub encryption_mode: String,
    pub include: BTreeSet<path::PathBuf>,
    pub exclude: BTreeSet<Pattern>,
    pub last_run: Option<RunInfo>,
}

fn fake_repo_id() -> String {
    format!("-randomid-{}", glib::uuid_string_random().to_string())
}

impl BackupConfig {
    pub fn include_dirs(&self) -> Vec<path::PathBuf> {
        let mut dirs = Vec::new();

        for dir in &self.include {
            dirs.push(absolute(dir));
        }

        dirs
    }

    pub fn exclude_dirs_internal(&self) -> Vec<Pattern> {
        let mut dirs = Vec::new();

        for Pattern::PathPrefix(dir) in &self.exclude {
            dirs.push(Pattern::PathPrefix(absolute(dir)));
        }

        dirs.push(Pattern::PathPrefix(absolute(path::Path::new(
            crate::REPO_MOUNT_DIR,
        ))));

        dirs
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum Pattern {
    PathPrefix(path::PathBuf),
    //Fnmatch(path::PathBuf),
}

impl Pattern {
    pub fn selector(&self) -> String {
        match self {
            Self::PathPrefix(_) => "pp",
            //Self::Fnmatch(_) => "fm",
        }
        .to_string()
    }

    pub fn pattern(&self) -> String {
        match self {
            Self::PathPrefix(p) => p,
            //Self::Fnmatch(p) => p,
        }
        .to_string_lossy()
        .to_string()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RunInfo {
    pub end: DateTime<Local>,
    pub result: Result<borg::Stats, String>,
}

impl RunInfo {
    pub fn new(result: Result<borg::Stats, String>) -> Self {
        Self {
            end: Local::now(),
            result,
        }
    }
}

pub type Password = Zeroizing<Vec<u8>>;

impl BackupConfig {
    pub fn new(repo: BackupRepo, info: borg::List, encrypted: bool) -> Self {
        let mut include = std::collections::BTreeSet::new();
        include.insert("".into());
        let mut exclude = std::collections::BTreeSet::new();
        exclude.insert(Pattern::PathPrefix(".cache".into()));

        Self {
            config_version: crate::CONFIG_VERSION,
            id: glib::uuid_string_random().to_string(),
            repo,
            repo_id: info.repository.id,
            encrypted,
            encryption_mode: info.encryption.mode,
            include,
            exclude,
            last_run: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum BackupRepo {
    Local {
        path: path::PathBuf,
        #[serde(alias = "device")]
        drive_name: Option<String>,
        #[serde(alias = "label")]
        mount_name: Option<String>,
        volume_uuid: Option<String>,
        removable: bool,
        icon: Option<String>,
        icon_symbolic: Option<String>,
        settings: Option<BackupSettings>,
    },
    Remote {
        uri: String,
        settings: Option<BackupSettings>,
    },
}

impl BackupRepo {
    pub fn new_from_uri(uri: String) -> Self {
        BackupRepo::Remote {
            uri,
            settings: None,
        }
    }

    pub fn new_from_path(repo: &path::Path) -> Self {
        let repo_file = gio::File::new_for_path(&if repo.exists() {
            repo
        } else {
            // for new repos the repo directory itself usually doesn't exist
            repo.parent().unwrap_or(repo)
        });

        let none: Option<&gio::Cancellable> = None;
        let mount = repo_file.find_enclosing_mount(none).ok();
        debug!("Mount found: {:?} {:?} {:?}", &repo, &mount, repo_file);
        let drive = mount.as_ref().and_then(gio::Mount::get_drive);

        let volume_uuid = mount.as_ref().and_then(get_mount_uuid);

        let icon = mount
            .as_ref()
            .and_then(gio::Mount::get_icon)
            .as_ref()
            .and_then(gio::IconExt::to_string)
            .map(|x| x.to_string());
        let icon_symbolic = mount
            .as_ref()
            .and_then(gio::Mount::get_symbolic_icon)
            .as_ref()
            .and_then(gio::IconExt::to_string)
            .map(|x| x.to_string());

        BackupRepo::Local {
            path: repo.to_path_buf(),
            icon,
            icon_symbolic,
            mount_name: mount
                .as_ref()
                .and_then(gio::Mount::get_name)
                .map(Into::into),
            drive_name: drive
                .as_ref()
                .and_then(gio::Drive::get_name)
                .map(Into::into),
            removable: drive.as_ref().map_or(false, gio::Drive::is_removable),
            volume_uuid,
            settings: None,
        }
    }

    pub fn icon(&self) -> String {
        match self {
            Self::Local { icon, .. } => icon.clone().unwrap_or_else(|| String::from("folder")),
            Self::Remote { .. } => String::from("network-server"),
        }
    }
    pub fn icon_symbolic(&self) -> String {
        match self {
            Self::Local { icon_symbolic, .. } => icon_symbolic
                .clone()
                .unwrap_or_else(|| String::from("folder-symbolic")),
            Self::Remote { .. } => String::from("network-server-symbolic"),
        }
    }
    pub fn set_settings(&mut self, settings: Option<BackupSettings>) {
        *match self {
            Self::Local {
                ref mut settings, ..
            } => settings,
            Self::Remote {
                ref mut settings, ..
            } => settings,
        } = settings;
    }

    pub fn get_settings(&self) -> Option<BackupSettings> {
        match self {
            Self::Local { settings, .. } => settings,
            Self::Remote { settings, .. } => settings,
        }
        .clone()
    }
}

impl std::fmt::Display for BackupRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let repo = match self {
            Self::Local { path, .. } => path.to_string_lossy().to_string(),
            Self::Remote { uri, .. } => uri.to_string(),
        };
        write!(f, "{}", repo)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BackupSettings {
    pub command_line_args: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct Settings {
    pub backups: BTreeMap<String, BackupConfig>,
}

pub fn get_home_dir() -> path::PathBuf {
    crate::globals::HOME_DIR.to_path_buf()
}

pub fn absolute(path: &path::Path) -> path::PathBuf {
    get_home_dir().join(path)
}

pub fn get_mount_uuid(mount: &gio::Mount) -> Option<String> {
    let volume = mount.get_volume();

    volume
        .as_ref()
        .and_then(gio::Volume::get_uuid)
        .or_else(|| volume.as_ref().and_then(|v| v.get_identifier("uuid")))
        .as_ref()
        .map(std::string::ToString::to_string)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
//#[serde(rename_all = "snake_case")]
pub enum Progress {
    #[serde(rename = "archive_progress")]
    Archive {
        original_size: u64,
        compressed_size: u64,
        deduplicated_size: u64,
        nfiles: u64,
        path: String,
    },
    #[serde(rename = "progress_message")]
    Message {
        operation: u64,
        msgid: Option<String>,
        finished: bool,
        message: Option<String>,
    },
    #[serde(rename = "progress_percent")]
    Percent {
        operation: u64,
        msgid: Option<String>,
        finished: bool,
        message: Option<String>,
        current: Option<u64>,
        total: Option<u64>,
    },
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub enum LogLevel {
    DEBUG,
    INFO,
    WARNING,
    ERROR,
    CRITICAL,
}

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub enum MsgId {
    ConnectionClosed,
    ConnectionClosedWithHint,
    PassphraseWrong,
    #[serde(rename = "Repository.DoesNotExist")]
    RepositoryDoesNotExist,
    Other(String),
    #[serde(other)]
    Undefined,
}

impl Default for MsgId {
    fn default() -> Self {
        Self::Undefined
    }
}

#[derive(Deserialize)]
pub struct MsgIdHelper {
    pub msgid: String,
}

#[derive(Clone, Debug)]
pub enum LogMessageEnum {
    ParsedErr(LogMessage),
    UnparsableErr(String),
}

impl LogMessageEnum {
    pub fn message(&self) -> String {
        match &self {
            Self::ParsedErr(LogMessage { ref message, .. }) => message.to_string(),
            Self::UnparsableErr(ref message) => message.to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct LogMessageCollection {
    pub messages: Vec<LogMessageEnum>,
}

impl LogMessageCollection {
    pub fn new(messages: Vec<LogMessageEnum>) -> Self {
        Self { messages }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub struct LogMessage {
    pub levelname: LogLevel,
    pub name: String,
    pub message: String,
    #[serde(default)]
    pub msgid: MsgId,
}

impl std::fmt::Display for LogMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for LogMessage {}

#[derive(Debug)]
pub struct BorgUnparsableErr {
    pub stderr: String,
}

impl std::fmt::Display for BorgUnparsableErr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "STDERR({})", self.stderr)
    }
}

impl std::fmt::Display for LogMessageEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::ParsedErr(e) => write!(f, "{}", e),
            Self::UnparsableErr(s) => write!(f, "{}", s),
        }
    }
}

impl std::fmt::Display for LogMessageCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.messages
                .iter()
                .map(|m| format!("{}", &m))
                .collect::<Vec<String>>()
                .join("\n")
        )
    }
}

impl std::error::Error for LogMessageCollection {}

#[derive(Debug)]
pub struct ReturnCodeErr {
    pub code: Option<i32>,
}

impl ReturnCodeErr {
    pub fn new(code: Option<i32>) -> Self {
        Self { code }
    }
}

impl std::fmt::Display for ReturnCodeErr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Return code err: {:?}", self.code)
    }
}

impl std::error::Error for ReturnCodeErr {}

quick_error! {
    #[derive(Debug)]
    pub enum BorgErr {
        Io(err: std::io::Error) { from() }
        Json(err: serde_json::error::Error) { from () }
        Unix(err: nix::Error) { from() }
        Borg(err: LogMessageCollection) {
            from()
            display("{}", err)
        }
        BorgCode(err: ReturnCodeErr) { from() }
        PasswordMissing { from(secret_service::SsError) }
        UserAborted {}
        Other(err: String) { from() }
    }
}

impl LogMessageEnum {
    pub fn has_borg_msgid(&self, msgid_needle: &MsgId) -> bool {
        if let Self::ParsedErr(x) = self {
            if x.msgid == *msgid_needle {
                return true;
            }
        }

        false
    }
}

impl BorgErr {
    pub fn has_borg_msgid(&self, msgid_needle: &MsgId) -> bool {
        match self {
            Self::Borg(LogMessageCollection { messages }) => {
                for message in messages {
                    if message.has_borg_msgid(msgid_needle) {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }
}
