#[macro_use]
extern crate matches;

use pika_backup::{
    borg, borg::msg::*, borg::prelude::*, borg::Borg, borg::BorgOnlyRepo, config, prelude::*,
};

// Currently, there are no init tasks
fn init() {}

#[test]
#[ignore]
fn borg_bin_missing() {
    init();
    std::env::set_var("PATH", "");
    let result = Borg::new(config()).create(status());
    assert_matches!(result, Err(borg::Error::Io(std::io::Error { .. })));
}

#[test]
fn simple_backup() {
    init();
    let borg = Borg::new(config());
    assert_matches!(borg.init(), Ok(borg::List { .. }));
    assert_matches!(borg.create(status()), Ok(_));
}

#[test]
fn backup_communication() -> borg::Result<()> {
    let borg = Borg::new(config());
    let communication = status();

    borg.init()?;
    borg.create(communication.clone())?;

    assert_matches!(communication.status.get().run, borg::Run::Running);

    Ok(())
}

#[test]
fn encrypted_backup() {
    init();
    let mut config = config();
    config.encrypted = true;

    let mut borg = Borg::new(config);
    borg.set_password(b"x".to_vec().into());

    assert_matches!(borg.init(), Ok(borg::List { .. }));

    borg.unset_password();
    assert_matches!(borg.create(status()), Err(borg::Error::PasswordMissing));
}

#[test]
fn failed_ssh_connection() {
    init();
    let repo = config::BackupRepo::new_remote("ssh://backup.server.invalid/repo".to_string());

    let result = BorgOnlyRepo::new(repo).peek();
    assert!(result
        .unwrap_err()
        .has_borg_msgid(&MsgId::ConnectionClosedWithHint));
}

#[test]
fn failed_repo() {
    init();
    let result = borg::Borg::new(config()).peek();
    assert!(result
        .unwrap_err()
        .has_borg_msgid(&MsgId::RepositoryDoesNotExist));
}

fn status() -> borg::Communication {
    Default::default()
}

fn config() -> config::Backup {
    let uuid = glib::uuid_string_random().to_string();
    let path = std::path::PathBuf::from(format!("/tmp/{}", &uuid));
    config::Backup {
        config_version: 1,
        id: ConfigId::new(uuid),
        repo_id: borg::RepoId::new("repo id".into()),
        encryption_mode: "none".into(),
        repo: config::BackupRepo::new_local_from_path(path),
        encrypted: false,
        include: vec!["/dev/null".into()].into_iter().collect(),
        exclude: Default::default(),
        last_run: None,
    }
}
