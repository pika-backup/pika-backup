#[macro_use]
extern crate matches;

use pika_backup::{borg, borg::Borg, shared, ui::prelude::*};
use shared::*;

type CheckError = Result<(), Box<dyn std::error::Error>>;

fn init() {
    pretty_env_logger::try_init().unwrap_or_default();
}

#[test]
#[ignore]
fn borg_bin_missing() {
    init();
    std::env::set_var("PATH", "");
    let result = Borg::new(config()).create(status());
    assert_matches!(
        result,
        Err(BorgErr::Io(std::io::Error { .. }))
    );
}

#[test]
fn simple_backup() {
    init();
    let borg = Borg::new(config());
    assert_matches!(borg.init(), Ok(()));
    assert_matches!(borg.create(status()), Ok(_));
}

#[test]
fn backup_communication() -> CheckError {
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

    assert_matches!(borg.init(), Ok(()));

    borg.unset_password();
    assert_matches!(borg.create(status()), Err(BorgErr::PasswordMissing));
}

#[test]
fn failed_ssh_connection() {
    init();
    let config = shared::BackupConfig::new_from_uri("ssh://backup.server.invalid/repo".to_string());

    let result = Borg::new(config).info();
    assert!(result
        .unwrap_err()
        .has_borg_msgid(&MsgId::ConnectionClosedWithHint));
}

#[test]
fn failed_repo() {
    init();
    let result = borg::Borg::new(config()).info();
    assert!(result
        .unwrap_err()
        .has_borg_msgid(&MsgId::RepositoryDoesNotExist));
}

fn status() -> borg::Communication {
    Default::default()
}

fn config() -> shared::BackupConfig {
    let uuid = glib::uuid_string_random().unwrap().to_string();
    shared::BackupConfig {
        id: uuid.clone(),
        repo: shared::BackupRepo::Local {
            path: format!("/tmp/{}", &uuid).into(),
            icon: None,
            label: None,
            device: None,
            removable: false,
            volume_uuid: None,
        },
        encrypted: false,
        include: vec!["/dev/null".into()].into_iter().collect(),
        exclude: Default::default(),
        last_run: None,
    }
}
